//! COV-067: check that a feature's coverage mappings point at test ranges
//! carrying each scenario's `@step` comments.

use std::path::Path;

/// Every `scenario: step` of `feature` whose `// @step <step>` comment is
/// not inside any of that scenario's mapped test ranges in
/// `<feature>.coverage`. Paths in the coverage file are relative to
/// `repository`. An unreadable or malformed file is an error.
pub fn unmatched_steps(repository: &Path, feature: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(feature)
        .map_err(|error| format!("cannot read {}: {error}", feature.display()))?;
    let coverage_path = feature.with_extension("feature.coverage");
    let coverage: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&coverage_path)
            .map_err(|error| format!("cannot read {}: {error}", coverage_path.display()))?,
    )
    .map_err(|error| format!("{} is not valid JSON: {error}", coverage_path.display()))?;
    let mut unmatched = Vec::new();
    for (scenario, steps) in scenarios(&text) {
        let ranges = mapped_ranges(&coverage, &scenario)?;
        let mut mapped = Vec::new();
        for (file, start, end) in ranges {
            let source = std::fs::read_to_string(repository.join(&file))
                .map_err(|error| format!("cannot read {file}: {error}"))?;
            mapped.extend(
                source
                    .lines()
                    .skip(start.saturating_sub(1))
                    .take(end + 1 - start)
                    .map(|line| normalise(line.trim())),
            );
        }
        for step in steps {
            let comment = normalise(&format!("// @step {step}"));
            if !mapped.iter().any(|line| line == &comment) {
                unmatched.push(format!("{scenario}: {step}"));
            }
        }
    }
    Ok(unmatched)
}

fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `(scenario name, steps)` for every scenario in a Gherkin file.
fn scenarios(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix("Scenario: ") {
            out.push((name.to_string(), Vec::new()));
        } else if ["Given ", "When ", "Then ", "And ", "But "]
            .iter()
            .any(|keyword| line.starts_with(keyword))
        {
            if let Some((_, steps)) = out.last_mut() {
                steps.push(line.to_string());
            }
        }
    }
    out
}

/// The `(file, start, end)` test ranges mapped for `scenario`.
fn mapped_ranges(
    coverage: &serde_json::Value,
    scenario: &str,
) -> Result<Vec<(String, usize, usize)>, String> {
    let entry = coverage["scenarios"]
        .as_array()
        .ok_or("the coverage file has no scenarios array")?
        .iter()
        .find(|entry| entry["name"] == scenario)
        .ok_or_else(|| format!("scenario '{scenario}' is not in the coverage file"))?;
    let mappings = entry["testMappings"]
        .as_array()
        .ok_or_else(|| format!("scenario '{scenario}' has no test mappings"))?;
    mappings
        .iter()
        .map(|mapping| {
            let file = mapping["file"]
                .as_str()
                .ok_or_else(|| format!("a mapping of '{scenario}' has no file"))?;
            let lines = mapping["lines"]
                .as_str()
                .ok_or_else(|| format!("a mapping of '{scenario}' has no line range"))?;
            let (start, end) = lines
                .split_once('-')
                .ok_or_else(|| format!("'{lines}' is not a start-end range"))?;
            let parse = |value: &str| {
                value
                    .parse::<usize>()
                    .map_err(|error| format!("'{lines}' is not a line range: {error}"))
            };
            Ok((file.to_string(), parse(start)?, parse(end)?))
        })
        .collect()
}
