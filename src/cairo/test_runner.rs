use cairo_lang_compiler::{db::RootDatabase, diagnostics::DiagnosticsReporter};
use cairo_lang_runner::{RunResultValue, SierraCasmRunner, StarknetExecutionResources};
use cairo_lang_sierra::program::Program;
use cairo_lang_test_plugin::{
    compile_test_prepared_db,
    test_config::{PanicExpectation, TestExpectation},
    TestConfig, TestsCompilationConfig,
};
use cairo_lang_test_runner::{RunProfilerConfig, TestRunConfig};

use std::sync::Mutex;

use anyhow::{Context, Result};
use cairo_lang_runner::ProfilingInfoCollectionConfig;
use cairo_lang_starknet::contract::ContractInfo;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
// use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use starknet_types_core::felt::Felt as Felt252;

use crate::cairo::runner::setup_input_string_project;

pub struct TestsSummary {
    passed: Vec<String>,
    failed: Vec<String>,
    failed_run_results: Vec<RunResultValue>,
}
enum TestStatus {
    Success,
    Fail(RunResultValue),
}

/// The result of a ran test.
struct TestResult {
    /// The status of the run.
    status: TestStatus,
    /// The gas usage of the run if relevant.
    gas_usage: Option<i64>,
    /// The used resources of the run.
    used_resources: StarknetExecutionResources,
    // /// The profiling info of the run, if requested.
    // profiling_info: Option<ProfilingInfo>,
}

pub fn run_cairo_tests(code: String) -> anyhow::Result<Option<TestsSummary>> {
    let mut db_builder = RootDatabase::builder();
    db_builder.detect_corelib();
    let db = &mut db_builder.build()?;
    let crate_id = setup_input_string_project(db, code)?;

    let compiled = compile_test_prepared_db(
        db,
        TestsCompilationConfig {
            starknet: true,
            add_statements_functions: false,
            add_statements_code_locations: false,
            contract_declarations: None,
            contract_crate_ids: None,
            executable_crate_ids: None,
        },
        vec![crate_id],
        DiagnosticsReporter::stderr().with_crates(&[crate_id]),
    )?;

    let config = TestRunConfig {
        filter: "".into(),
        include_ignored: false,
        ignored: false,
        run_profiler: RunProfilerConfig::None,
        gas_enabled: false,
        print_resource_usage: false,
    };

    let TestsSummary {
        passed,
        failed,
        failed_run_results,
    } = run_tests(
        compiled.metadata.named_tests,
        compiled.sierra_program.program,
        compiled.metadata.contracts_info,
        &config,
    )?;
    todo!()
}

/// Updates the test summary with the given test result.
fn update_summary(
    wrapped_summary: &Mutex<std::prelude::v1::Result<TestsSummary, anyhow::Error>>,
    test_result: std::prelude::v1::Result<(String, Option<TestResult>), anyhow::Error>,
    sierra_program: &Program,
    print_resource_usage: bool,
) {
    let mut wrapped_summary = wrapped_summary.lock().unwrap();
    if wrapped_summary.is_err() {
        return;
    }
    let (name, opt_result) = match test_result {
        Ok((name, opt_result)) => (name, opt_result),
        Err(err) => {
            *wrapped_summary = Err(err);
            return;
        }
    };
    let summary = wrapped_summary.as_mut().unwrap();
    let mut empty_tests: Vec<String> = vec![];
    let (res_type, status_str, gas_usage) = if let Some(result) = opt_result {
        let (res_type, status_str) = match result.status {
            TestStatus::Success => (&mut summary.passed, "ok"),
            TestStatus::Fail(run_result) => {
                summary.failed_run_results.push(run_result);
                (&mut summary.failed, "fail")
            }
        };
        (res_type, status_str, result.gas_usage)
    } else {
        (&mut empty_tests, "ignored", None)
    };
    if let Some(gas_usage) = gas_usage {
        println!("test {name} ... {status_str} (gas usage est.: {gas_usage})");
    } else {
        println!("test {name} ... {status_str}");
    }

    res_type.push(name);
}

/// Runs the tests and process the results for a summary.
pub fn run_tests(
    named_tests: Vec<(String, TestConfig)>,
    sierra_program: Program,
    contracts_info: OrderedHashMap<Felt252, ContractInfo>,
    config: &TestRunConfig,
) -> Result<TestsSummary> {
    let runner = SierraCasmRunner::new(
        sierra_program.clone(),
        None,
        contracts_info,
        match config.run_profiler {
            RunProfilerConfig::None => None,
            RunProfilerConfig::Cairo | RunProfilerConfig::Sierra => {
                Some(ProfilingInfoCollectionConfig::default())
            }
        },
    )
    .with_context(|| "Failed setting up runner.")?;
    let suffix = if named_tests.len() != 1 { "s" } else { "" };
    println!("running {} test{}", named_tests.len(), suffix);
    let wrapped_summary = Mutex::new(Ok(TestsSummary {
        passed: vec![],
        failed: vec![],
        failed_run_results: vec![],
    }));

    // Run in parallel if possible. If running with db, parallelism is impossible.
    named_tests
        .into_iter()
        .map(move |(name, test)| run_single_test(test, name, &runner))
        .for_each(|test_result| {
            update_summary(
                &wrapped_summary,
                test_result,
                &sierra_program,
                config.print_resource_usage,
            );
        });

    wrapped_summary.into_inner().unwrap()
}

/// Runs a single test and returns a tuple of its name and result.
fn run_single_test(
    test: TestConfig,
    name: String,
    runner: &SierraCasmRunner,
) -> anyhow::Result<(String, Option<TestResult>)> {
    if test.ignored {
        return Ok((name, None));
    }
    let func = runner.find_function(name.as_str())?;
    let result = runner
        .run_function_with_starknet_context(func, vec![], test.available_gas, Default::default())
        .with_context(|| format!("Failed to run the function `{}`.", name.as_str()))?;
    Ok((
        name,
        Some(TestResult {
            status: match &result.value {
                RunResultValue::Success(_) => match test.expectation {
                    TestExpectation::Success => TestStatus::Success,
                    TestExpectation::Panics(_) => TestStatus::Fail(result.value),
                },
                RunResultValue::Panic(value) => match test.expectation {
                    TestExpectation::Success => TestStatus::Fail(result.value),
                    TestExpectation::Panics(panic_expectation) => match panic_expectation {
                        PanicExpectation::Exact(expected) if value != &expected => {
                            TestStatus::Fail(result.value)
                        }
                        _ => TestStatus::Success,
                    },
                },
            },
            gas_usage: Some(result.gas_counter.unwrap().to_bigint().to_u64_digits().1[0] as i64),
            used_resources: result.used_resources,
            // profiling_info: result.profiling_info,
        }),
    ))
}
