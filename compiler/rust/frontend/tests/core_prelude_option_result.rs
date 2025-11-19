//! Core Prelude (`Option`/`Result`) の 16 シナリオスナップショットテスト。
use reml_runtime_ffi::core_prelude::{Option as RemlOption, Result as RemlResult, Try};

fn push_case(cases: &mut Vec<(String, String)>, name: &str, value: String) {
    cases.push((name.to_string(), value));
}

#[test]
fn option_result_snapshot() {
    let mut cases = Vec::new();

    let option_is_some = format!(
        "Some({}) | None({})",
        RemlOption::Some(5).is_some(),
        RemlOption::<i32>::None.is_some()
    );
    push_case(&mut cases, "option_is_some", option_is_some);

    let option_map = format!(
        "{:?} | {:?}",
        RemlOption::Some(10).map(|v| v * 2),
        RemlOption::<i32>::None.map(|v| v * 2)
    );
    push_case(&mut cases, "option_map", option_map);

    let option_and_then = format!(
        "{:?} | {:?}",
        RemlOption::Some(3).and_then(|v| RemlOption::Some(v * 3)),
        RemlOption::<i32>::None.and_then(|v| RemlOption::Some(v))
    );
    push_case(&mut cases, "option_and_then", option_and_then);

    let option_ok_or = format!(
        "{:?} | {:?}",
        RemlOption::Some(7).ok_or(|| "missing user"),
        RemlOption::<i32>::None.ok_or(|| "missing user")
    );
    push_case(&mut cases, "option_ok_or", option_ok_or);

    let option_unwrap_or = format!(
        "{} | {}",
        RemlOption::Some(5).unwrap_or(0),
        RemlOption::<i32>::None.unwrap_or(0)
    );
    push_case(&mut cases, "option_unwrap_or", option_unwrap_or);

    push_case(
        &mut cases,
        "option_expect",
        RemlOption::Some(42).expect("value required").to_string(),
    );

    let option_try_branch = format!(
        "{:?} | {:?}",
        RemlOption::Some(3).branch(),
        RemlOption::<i32>::None.branch()
    );
    push_case(&mut cases, "option_try_branch", option_try_branch);

    let option_result_bridge = format!(
        "{:?} | {:?}",
        RemlOption::Some(8).ok_or(|| "missing label").to_option(),
        RemlOption::<i32>::None
            .ok_or(|| "missing label")
            .to_option()
    );
    push_case(&mut cases, "option_result_bridge", option_result_bridge);

    let result_map = format!(
        "{:?} | {:?}",
        RemlResult::<i32, &str>::Ok(5).map(|v| v * 2),
        RemlResult::<i32, &str>::Err("boom").map(|v| v * 2)
    );
    push_case(&mut cases, "result_map", result_map);

    let result_map_err = format!(
        "{:?}",
        RemlResult::<i32, &str>::Err("boom").map_err(|e| format!("{e}:converted"))
    );
    push_case(&mut cases, "result_map_err", result_map_err);

    let result_and_then = format!(
        "{:?} | {:?}",
        RemlResult::<i32, &str>::Ok(3).and_then(|v| RemlResult::<i32, &str>::Ok(v * 5)),
        RemlResult::<i32, &str>::Err("boom").and_then(|v| RemlResult::<i32, &str>::Ok(v * 5))
    );
    push_case(&mut cases, "result_and_then", result_and_then);

    let result_or_else = format!(
        "{:?} | {:?}",
        RemlResult::<i32, &str>::Ok(8).or_else(|_| RemlResult::<i32, &str>::Ok(0)),
        RemlResult::<i32, &str>::Err("boom").or_else(|_| RemlResult::<i32, &str>::Ok(64))
    );
    push_case(&mut cases, "result_or_else", result_or_else);

    let result_unwrap_or = format!(
        "{} | {}",
        RemlResult::<i32, &str>::Ok(88).unwrap_or(0),
        RemlResult::<i32, &str>::Err("boom").unwrap_or(0)
    );
    push_case(&mut cases, "result_unwrap_or", result_unwrap_or);

    push_case(
        &mut cases,
        "result_expect",
        RemlResult::<i32, &str>::Ok(13)
            .expect("value expected")
            .to_string(),
    );

    let result_to_option = format!(
        "{:?} | {:?}",
        RemlResult::<i32, &str>::Ok(21).to_option(),
        RemlResult::<i32, &str>::Err("boom").to_option()
    );
    push_case(&mut cases, "result_to_option", result_to_option);

    let result_from_option = format!(
        "{:?} | {:?}",
        RemlResult::from_option(RemlOption::Some(9), "missing id"),
        RemlResult::from_option(RemlOption::<i32>::None, "missing id")
    );
    push_case(&mut cases, "result_from_option", result_from_option);

    let actual = cases
        .into_iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("\n");
    const SNAPSHOT: &str = include_str!("core_prelude_option_result.snap");
    let expected = SNAPSHOT.trim_end_matches('\n');
    assert_eq!(
        actual, expected,
        "Core Prelude Option/Result snapshot が変化しました"
    );
}
