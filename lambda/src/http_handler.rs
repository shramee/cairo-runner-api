use cairo_runners::{main_runner::run_cairo_code, test_runner::run_cairo_tests};
use lambda_http::{Body, Error, Request, RequestExt, Response};

fn run_cairo_tests_get_notes(code: String) -> String {
    match run_cairo_tests(code) {
        Ok(res) => res.notes().to_string(),
        Err(e) => format!("{e}"),
    }
}

fn run_cairo_main_get_notes(code: String) -> String {
    match run_cairo_code(code) {
        Ok(res) => res.to_string(),
        Err(e) => format!("{e}"),
    }
}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
pub(crate) async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    let code = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("code"))
        .unwrap_or_default();

    let result = match event
        .query_string_parameters_ref()
        .and_then(|params| params.first("test"))
    {
        // run tests if the `test` query parameter is present
        Some(_) => run_cairo_tests_get_notes(code.to_string()),
        // otherwise run the main function
        None => run_cairo_main_get_notes(code.to_string()),
    };

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/json")
        .body(result.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::{Request, RequestExt};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_empty_code() {
        let request = Request::default();

        let response = function_handler(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body_bytes = response.body().to_vec();
        let body_string = String::from_utf8(body_bytes).unwrap();

        assert!(body_string.contains("Function with suffix `::main` to run not found."));
    }

    #[tokio::test]
    async fn test_main_runner() {
        let mut query_string_parameters: HashMap<String, String> = HashMap::new();
        query_string_parameters.insert("code".into(), "fn main() -> felt252 {0x25}".into());

        let request = Request::default().with_query_string_parameters(query_string_parameters);

        let response = function_handler(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body_bytes = response.body().to_vec();
        let body_string = String::from_utf8(body_bytes).unwrap();

        assert!(body_string.contains("Run completed successfully, returning"));
    }

    #[tokio::test]
    async fn test_test_runner() {
        let mut query_string_parameters: HashMap<String, String> = HashMap::new();
        query_string_parameters.insert("test".into(), "".into());
        query_string_parameters.insert(
            "code".into(),
            r#"
    #[test]
    fn test_pass() {assert(true, 'should pass');}
    #[test]
    fn test_fail() {assert(false, 'should fail');}
    "#
            .into(),
        );

        let request = Request::default().with_query_string_parameters(query_string_parameters);

        let response = function_handler(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body_bytes = response.body().to_vec();
        let body_string = String::from_utf8(body_bytes).unwrap();

        assert!(body_string.contains("running 2 tests"));
        assert!(body_string.contains("test lib::test_pass ... ok"));
        assert!(body_string.contains("test lib::test_fail ... fail"));
    }
}
