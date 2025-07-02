use cairo_lang_compiler::{db::RootDatabase, diagnostics::DiagnosticsReporter};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_runner::{RunResultValue, SierraCasmRunner};
use cairo_lang_sierra::program::Program;
use cairo_lang_sierra_to_casm::metadata::MetadataComputationConfig;
use cairo_lang_test_plugin::{
    compile_test_prepared_db,
    test_config::{PanicExpectation, TestExpectation},
    test_plugin_suite, TestConfig, TestsCompilationConfig,
};
use cairo_lang_test_runner::{RunProfilerConfig, TestRunConfig};

use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use cairo_lang_runner::ProfilingInfoCollectionConfig;
use cairo_lang_starknet::{contract::ContractInfo, starknet_plugin_suite};
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
// use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use starknet_types_core::felt::Felt as Felt252;

use crate::main_runner::setup_input_string_project;

#[derive(Debug)]
pub struct TestsSummary {
    passed: Vec<String>,
    failed: Vec<String>,
    failed_run_results: Vec<RunResultValue>,
    notes: String,
}

impl TestsSummary {
    pub fn passed(&self) -> &[String] {
        &self.passed
    }
    pub fn failed(&self) -> &[String] {
        &self.failed
    }
    pub fn failed_run_results(&self) -> &[RunResultValue] {
        &self.failed_run_results
    }
    pub fn notes(&self) -> &str {
        &self.notes
    }
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
    // /// The used resources of the run.
    // used_resources: StarknetExecutionResources,
    // /// The profiling info of the run, if requested.
    // profiling_info: Option<ProfilingInfo>,
}

pub fn run_cairo_tests(code: String) -> anyhow::Result<TestsSummary> {
    let cfg = CfgSet::from_iter([Cfg::name("test"), Cfg::kv("target", "test")]);
    let mut db_builder = RootDatabase::builder();
    db_builder.detect_corelib();
    db_builder.with_cfg(cfg);
    db_builder.with_default_plugin_suite(test_plugin_suite());
    db_builder.with_default_plugin_suite(starknet_plugin_suite());
    let db = &mut db_builder.build()?;

    let crate_id = setup_input_string_project(db, code)?;

    let mut diagnostics = String::new();

    let compiled = match compile_test_prepared_db(
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
        DiagnosticsReporter::callback(|diagnostic| {
            diagnostics += format!("{}\n", diagnostic).as_str()
        }),
    ) {
        Ok(compiled) => compiled,
        Err(err) => {
            bail!("{err}\n\n{diagnostics}")
        }
    };

    let config = TestRunConfig {
        filter: "".into(),
        include_ignored: false,
        ignored: false,
        run_profiler: RunProfilerConfig::None,
        gas_enabled: false,
        print_resource_usage: false,
    };

    Ok(run_tests(
        compiled.metadata.named_tests,
        compiled.sierra_program.program,
        compiled.metadata.contracts_info,
        &config,
    )?)
}

/// Updates the test summary with the given test result.
fn update_summary(
    wrapped_summary: &Mutex<std::prelude::v1::Result<TestsSummary, anyhow::Error>>,
    test_result: std::prelude::v1::Result<(String, Option<TestResult>), anyhow::Error>,
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
        summary.notes += &format!("\ntest {name} ... {status_str} (gas usage est.: {gas_usage})");
    } else {
        summary.notes += &format!("\ntest {name} ... {status_str}");
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
        Some(MetadataComputationConfig::default()),
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
    let notes = format!("running {} test{}", named_tests.len(), suffix);
    let wrapped_summary = Mutex::new(Ok(TestsSummary {
        passed: vec![],
        failed: vec![],
        failed_run_results: vec![],
        notes,
    }));

    // Run in parallel if possible. If running with db, parallelism is impossible.
    named_tests
        .into_iter()
        .map(move |(name, test)| run_single_test(test, name, &runner))
        .for_each(|test_result| {
            update_summary(&wrapped_summary, test_result);
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
            gas_usage: result
                .gas_counter
                .map(|f| *f.to_bigint().to_u64_digits().1.first().unwrap_or(&0) as i64),
            // used_resources: result.used_resources,
            // profiling_info: result.profiling_info,
        }),
    ))
}

#[cfg(test)]
mod test_runner_tests {
    use super::*;

    #[test]
    fn fail_compilation() {
        let code = r#"fn main(){}{"#;

        match run_cairo_tests(code.to_string()) {
            Ok(_) => {
                panic!("Error compilation should fail")
            }
            Err(e) => {
                assert!(e.to_string().contains("Compilation failed"));
            }
        }
    }

    #[test]
    fn no_tests() {
        let code = r#"
fn main(){// this is some Cairo code
}"#;

        match run_cairo_tests(code.to_string()) {
            Ok(output) => {
                assert!(
                    output.failed.is_empty()
                        && output.passed.is_empty()
                        && output.failed_run_results.is_empty()
                );
            }
            Err(e) => panic!("Error: {}", e),
        }
    }

    #[test]
    fn diagnostics_test() {
        let code = r#"
    #[test]
    fn test_failing() {
        assert(false, "fail diagnostics");
    }
    "#;
        match run_cairo_tests(code.to_string()) {
            Ok(_) => panic!("Error: test tunner should fail with diagnostics"),
            Err(e) => {
                let e = e.to_string();
                assert!(e.contains("Mismatched types. The type `core::felt252` cannot be created from a string literal."));
                assert!(e
                    .to_string()
                    .contains("assert(false, \"fail diagnostics\");"));
                assert!(e.contains("  ^^^^^^^^^^^^^^^^^")); // some sorta pointer
            }
        }
    }

    #[test]
    fn test_panic() {
        let code = r#"
    #[test]
    fn test_failing() {
    	assert(false, 'should fail');
    }
    "#;
        match run_cairo_tests(code.to_string()) {
            Ok(output) => {
                assert!(output.passed.is_empty());
                assert!(output.failed.len() == 1);
                assert!(output.failed_run_results.len() == 1);
                assert!(output.notes.contains("test lib::test_failing ... fail"));
            }
            Err(e) => panic!("Error: {}", e),
        }
    }

    #[test]
    fn test_pass() {
        let code = r#"
    #[test]
    fn test_pass() {
    	assert(true, 'should pass');
    }
    "#;
        match run_cairo_tests(code.to_string()) {
            Ok(output) => {
                assert!(output.passed.is_empty());
                assert!(output.failed.len() == 1);
                assert!(output.failed_run_results.len() == 1);
                assert!(output.notes.contains("test lib::test_failing ... fail"));
            }
            Err(e) => panic!("Error: {}", e),
        }
    }

    #[test]
    fn test_should_panic() {
        let code = r#"
    #[test]
    #[should_panic]
    fn test_should_panic() {
    	assert(false, 'panics');
    }
    "#;
        match run_cairo_tests(code.to_string()) {
            Ok(output) => {
                assert!(output.passed.len() == 1);
                assert!(output.failed.is_empty());
                assert!(output.failed_run_results.is_empty());
                assert!(output.notes.contains("test lib::test_should_panic ... ok"));
            }
            Err(e) => panic!("Error: {}", e),
        }
    }

    #[test]
    fn test_starknet() {
        let code = r#"#[starknet::interface]
trait IJoesContract<TContractState> {
    fn get_owner(self: @TContractState) -> felt252;
}

#[starknet::contract]
mod JoesContract {
    #[storage]struct Storage {}
    #[abi(embed_v0)]
    impl IJoesContractImpl of super::IJoesContract<ContractState> {
        fn get_owner(self: @ContractState) -> felt252 { 'Joe' }
    }
}

#[cfg(test)]
mod test {
    use super::{JoesContract, IJoesContractDispatcher, IJoesContractDispatcherTrait};
    #[test]
    #[available_gas(2000000000)]
    fn test_contract_view() {
        let (address0, _) = starknet::syscalls::deploy_syscall(
            JoesContract::TEST_CLASS_HASH.try_into().unwrap(), 0, [].span(), false
        ).unwrap();
        let contract = IJoesContractDispatcher { contract_address: address0 };
        assert('Joe' == contract.get_owner(), 'Joe should be the owner.');
    }
}"#;
        match run_cairo_tests(code.to_string()) {
            Ok(res) => {
                assert!(res.passed.len() == 1);
                assert!(res.failed.is_empty());
                assert!(res.failed_run_results.is_empty());
                assert!(res.notes.contains("test_contract_view ... ok"));
            }
            Err(e) => println!("\n\nError: ```{}```\n\n", e),
        }
    }
}
