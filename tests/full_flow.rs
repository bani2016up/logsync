use logsync::{application::compare_logfiles, domain::LogFile, tui};

#[test]
fn renders_three_fixture_logs_as_expected() {
    let paths = [
        "assets/integration-log-1.log",
        "assets/integration-log-2.log",
        "assets/integration-log-3.log",
    ];
    let logs: Vec<_> = paths
        .into_iter()
        .map(|path| LogFile::from_file(path.to_owned()))
        .collect();
    let result = compare_logfiles(logs.iter().collect());

    insta::assert_snapshot!(tui::snapshot(&result, 140, 16));
}
