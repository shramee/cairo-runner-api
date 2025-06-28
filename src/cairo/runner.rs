use anyhow::Context;
use cairo_lang_compiler::{
    db::RootDatabase, diagnostics::DiagnosticsReporter, project::ProjectError,
};
use cairo_lang_diagnostics::ToOption;
use cairo_lang_filesystem::{
    db::{CrateConfiguration, FilesGroupEx},
    ids::{CrateId, Directory},
};
use cairo_lang_formatter::cairo_formatter::FormattableInput;
use cairo_lang_runner::{casm_run::format_next_item, SierraCasmRunner, StarknetState};
use cairo_lang_semantic::{db::SemanticGroup, test_utils::get_crate_semantic_diagnostics};
use cairo_lang_sierra_generator::{
    db::SierraGenGroup,
    program_generator::SierraProgramWithDebug,
    replace_ids::{DebugReplacer, SierraIdReplacer},
};
use cairo_lang_starknet::contract::{find_contracts, get_contracts_info};
use cairo_lang_utils::Upcast;
use std::{collections::BTreeMap, sync::Arc};

const MEMORY_OUTPUT: bool = false;

pub fn setup_input_string_project(
    db: &mut dyn SemanticGroup,
    input: String,
) -> Result<CrateId, ProjectError> {
    let crate_id = CrateId::plain(db, "lib");
    let file_id = FormattableInput::to_file_id(&input, db.as_files_group_mut()).unwrap();

    let dir = Directory::Virtual {
        files: BTreeMap::from([("lib.cairo".into(), file_id)]),
        dirs: BTreeMap::new(),
    };

    db.set_crate_config(crate_id, Some(CrateConfiguration::default_for_root(dir)));
    Ok(crate_id)
}

pub fn run_cairo_code(code: String) -> anyhow::Result<String> {
    let mut output = "".into();
    let mut db_builder = RootDatabase::builder();
    db_builder.detect_corelib();
    let db = &mut db_builder.build()?;

    let main_crate_id = setup_input_string_project(db, code).unwrap();

    let mut reporter = DiagnosticsReporter::stderr().with_crates(&[main_crate_id]);

    if reporter.check(db) {
        let semantic_errors = get_crate_semantic_diagnostics(db, main_crate_id).format(db);
        anyhow::bail!(semantic_errors);
    }

    let SierraProgramWithDebug {
        program: mut sierra_program,
        debug_info: _,
    } = Arc::unwrap_or_clone(
        db.get_sierra_program([main_crate_id.clone()].into())
            .to_option()
            .with_context(|| "Compilation failed without any diagnostics.")?,
    );

    let replacer = DebugReplacer { db };
    replacer.enrich_function_names(&mut sierra_program);

    let contracts = find_contracts((*db).upcast(), &[main_crate_id]);
    let contracts_info = get_contracts_info(db, contracts, &replacer)?;
    let sierra_program = replacer.apply(&sierra_program);

    let runner = SierraCasmRunner::new(sierra_program.clone(), None, contracts_info, None)
        .with_context(|| "Failed to create Sierra runner.")
        .unwrap();

    let result = runner
        .run_function_with_starknet_context(
            runner.find_function("::main")?,
            vec![],
            None,
            StarknetState::default(),
        )
        .with_context(|| "Failed to run the function.")
        .unwrap();

    match result.value {
        cairo_lang_runner::RunResultValue::Success(values) => {
            output += format!("Run completed successfully, returning {values:?}\n").as_str();
        }
        cairo_lang_runner::RunResultValue::Panic(values) => {
            output += format!("Run panicked with [").as_str();
            let mut felts = values.into_iter();
            let mut first = true;
            while let Some(item) = format_next_item(&mut felts) {
                if !first {
                    output += format!(", ").as_str();
                }
                first = false;
                output += format!("{}", item.quote_if_string()).as_str();
            }
            output += format!("].\n").as_str();
        }
    }

    if MEMORY_OUTPUT {
        output += format!("Full memory: [").as_str();
        for cell in &result.memory {
            match cell {
                None => output += format!("_, ").as_str(),
                Some(value) => output += format!("{value}, ").as_str(),
            }
        }
        output += format!("]").as_str();
    }

    Ok(output)
}

#[cfg(test)]
mod runner_tests {
    use super::*;

    pub fn run_cairo_code_string_output(code: String) -> String {
        match run_cairo_code(code) {
            Ok(output) => output,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[test]
    fn test_cairo_code_success1() {
        let code = r#"
fn main(){// this is some Cairo code
}"#;
        let output = run_cairo_code_string_output(code.to_string());
        assert!(output.contains("Run completed successfully, returning"));
    }

    #[test]
    fn test_cairo_code_success() {
        let code = r#"
            fn main() -> felt252 {
                return 42;
            }
        "#;
        let output = run_cairo_code_string_output(code.to_string());
        assert!(output.contains("Run completed successfully, returning"));
    }

    #[test]
    fn test_cairo_code_compile_error() {
        let code = r#"
            fn main() -> felt252 {
                4 + 5;
            }
        "#;
        let output = match run_cairo_code(code.to_string()) {
            Ok(_) => panic!("output should have error"),
            Err(e) => format!("{}", e),
        };
        println!("\n\n{}\n\n", output);
        assert!(output.contains("Unexpected return type."));
    }

    #[test]
    fn test_cairo_code_panic() {
        let code = r#"
            fn main() {
                panic!("good_error_has_occurred");
            }
        "#;
        let output = run_cairo_code_string_output(code.to_string());
        assert!(output.contains("Run panicked with"));
        assert!(output.contains("good_error_has_occurred"));
    }
}
