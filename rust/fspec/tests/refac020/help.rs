use sha2::{Digest, Sha256};

use crate::TestResult;

/// `original` is the captured TypeScript help (frozen under
/// `refac020/ts_original/`); the live fixtures under `fixtures/help/` follow
/// their configs (CLI-016).
pub struct HelpContract {
    pub command: &'static str,
    pub original: &'static str,
    pub expected: &'static str,
    hash: &'static str,
    additions: &'static str,
    /// Approved later edits to the original text (`from` → `to`, each `from`
    /// occurring exactly once), applied before the option additions.
    edits: &'static [(&'static str, &'static str)],
}

pub const CONTRACTS: &[HelpContract] = &[
    HelpContract {
        command: "link-coverage",
        original: include_str!("ts_original/link-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/link-coverage.txt"),
        hash: "a4935bc1d735b03304763602fb9a33da4789fab26c3930f3bbf117a270375f70",
        additions: "  --test-mapping-index <index>\n    Zero-based test mapping index for implementation-only linking; requires --test-file and --expected-scenario-hash\n  --expected-scenario-hash <hash>\n    Expected canonical scenario SHA-256 from show-coverage --mapping-json; required with --test-mapping-index\n  --replace\n    Replace the scenario's mappings of --test-file with one mapping holding --test-lines, keeping their implementation mappings, and check only the new lines (without it a second link adds another mapping); requires --test-lines\n",
        edits: &[
            // COV-079: implementation mappings record their line text;
            // COV-078: --test-file without --test-lines links the exact block.
            (
                "  • Step validation fails with helpful system-reminder showing exact text to add to test file\n",
                "  • Step validation fails with helpful system-reminder showing exact text to add to test file\n  • Implementation mappings record the text of their first and last line (firstLine, lastLine) so fspec audit detects code that moved; the first line must be non-blank and unique in the file, so start the range at a distinctive line such as a signature\n  • Without --test-lines and --impl-file, --test-file links the scenario's exact @step block (its @step comments in step order with no other @step comment between) when it occurs exactly once in the file; otherwise link-coverage fails naming how often it occurs\n",
            ),
            // AUDIT-003: fspec audit replaced audit-coverage.
            ("  fspec audit-coverage\n", "  fspec audit\n"),
            (
                "  • Step comments support both @step prefix (recommended) and plain format (backward compatible)\n",
                "  • Only @step comments count, and each must repeat its step's keyword and text exactly (runs of spaces count as one); plain comments such as // Given... do not\n  • Long step comments: end the @step line with a backslash and continue on the next comment line; the parts are joined with one space\n",
            ),
            (
                "  • Parameterized steps ({int}, {string}) match via hybrid similarity algorithm\n",
                "  • There is no similarity matching: a step comment with other words, stale wording or different letter case does not match\n",
            ),
            // BUG-566: a pattern without a description prints no `undefined` line.
            (
                "--impl-file src/auth/login.ts --impl-lines 10-24\n    undefined\n",
                "--impl-file src/auth/login.ts --impl-lines 10-24\n",
            ),
            (
                "--impl-lines 45-67 --skip-validation\n    undefined\n",
                "--impl-lines 45-67 --skip-validation\n",
            ),
        ],
    },
    HelpContract {
        command: "unlink-coverage",
        original: include_str!("ts_original/unlink-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/unlink-coverage.txt"),
        hash: "8d59620dcd586756e0dd2d02f82d781c7ada30034f47b4fd46e4b67d2204b395",
        additions: "  --test-mapping-index <index>\n    Zero-based test mapping index to unlink; requires --test-file and --expected-scenario-hash and cannot be combined with --all\n  --expected-scenario-hash <hash>\n    Expected canonical scenario SHA-256 from show-coverage --mapping-json; required with --test-mapping-index\n",
        // CLI-016: audit-coverage has no --fix; AUDIT-003: fspec audit replaced it.
        edits: &[
            // AUDIT-003: fspec audit replaced audit-coverage.
            ("  fspec audit-coverage\n", "  fspec audit\n"),
            (
                "# Or use audit-coverage --fix\nfspec audit-coverage user-auth --fix\n    undefined\n",
                "# Then confirm no broken mappings remain\nfspec audit --feature user-auth\n",
            ),
            // BUG-566: a pattern without a description prints no `undefined` line.
            (
                "--test-file <correct-path> --test-lines <range>\n    undefined\n",
                "--test-file <correct-path> --test-lines <range>\n",
            ),
            (
                "--impl-file src/auth/new-login.ts --impl-lines 5-15\n    undefined\n",
                "--impl-file src/auth/new-login.ts --impl-lines 5-15\n",
            ),
            (
                "  • Consider using audit-coverage --fix instead for batch cleanup\n",
                "  • Run fspec audit afterwards to confirm no broken mappings remain\n",
            ),
        ],
    },
    HelpContract {
        command: "generate-coverage",
        original: include_str!("ts_original/generate-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/generate-coverage.txt"),
        hash: "67fcfe7251948c87a3cfc1a5c65550bb1e269bf9beda7ec1a02cf945b9a16d64",
        additions: "  --feature-name <name>\n    Generate only the exact selected feature; return a JSON plan, preserve lineage, and block unsafe removals or invalid metadata. With --dry-run, preview the same plan without writes.\n",
        edits: &[
            // AUDIT-003: fspec audit replaced audit-coverage.
            ("  fspec audit-coverage\n", "  fspec audit\n"),
        ],
    },
    HelpContract {
        command: "show-coverage",
        original: include_str!("ts_original/show-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/show-coverage.txt"),
        hash: "95cb036ddbe8fdb0c3778e23013457e8fee68c8a38d055b2b43f8086a09d5f1c",
        additions: "  --mapping-json\n    Read ordered mapping indices and canonical scenario hashes as JSON; requires a feature name and leaves --format json unchanged\n",
        // BUG-566: a pattern without a description prints no `undefined` line.
        edits: &[
            // AUDIT-003: fspec audit replaced audit-coverage.
            ("  fspec audit-coverage\n", "  fspec audit\n"),
            (
                "# Link missing tests/implementation\n    undefined\n",
                "# Link missing tests/implementation\n",
            ),
            (
                "# Output: Overall Coverage: 45% (9/20 scenarios)\n    undefined\n",
                "# Output: Overall Coverage: 45% (9/20 scenarios)\n",
            ),
            (
                "jq '.coveragePercent' coverage.json\n    undefined\n",
                "jq '.coveragePercent' coverage.json\n",
            ),
        ],
    },
];

pub fn assert_originals() -> TestResult {
    for contract in CONTRACTS {
        assert_eq!(
            hex::encode(Sha256::digest(contract.original.as_bytes())),
            contract.hash
        );
        let boundary = "\nCOMMON PATTERNS\n";
        assert_eq!(contract.original.matches(boundary).count(), 1);
        let mut original = contract.original.to_string();
        for (from, to) in contract.edits {
            assert_eq!(original.matches(from).count(), 1, "{from}");
            original = original.replacen(from, to, 1);
        }
        let expected = original.replacen(
            boundary,
            &format!("{}\nCOMMON PATTERNS\n", contract.additions),
            1,
        );
        assert_eq!(
            contract.expected, expected,
            "{} fixture changed beyond its explicit approved option block",
            contract.command
        );
    }
    Ok(())
}
