use sha2::{Digest, Sha256};

use crate::TestResult;

pub struct HelpContract {
    pub command: &'static str,
    pub original: &'static str,
    pub expected: &'static str,
    hash: &'static str,
    additions: &'static str,
}

pub const CONTRACTS: &[HelpContract] = &[
    HelpContract {
        command: "link-coverage",
        original: include_str!("../fixtures/help/link-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/link-coverage.txt"),
        hash: "a4935bc1d735b03304763602fb9a33da4789fab26c3930f3bbf117a270375f70",
        additions: "  --test-mapping-index <index>\n    Zero-based test mapping index for implementation-only linking; requires --test-file and --expected-scenario-hash\n  --expected-scenario-hash <hash>\n    Expected canonical scenario SHA-256 from show-coverage --mapping-json; required with --test-mapping-index\n",
    },
    HelpContract {
        command: "unlink-coverage",
        original: include_str!("../fixtures/help/unlink-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/unlink-coverage.txt"),
        hash: "8d59620dcd586756e0dd2d02f82d781c7ada30034f47b4fd46e4b67d2204b395",
        additions: "  --test-mapping-index <index>\n    Zero-based test mapping index to unlink; requires --test-file and --expected-scenario-hash and cannot be combined with --all\n  --expected-scenario-hash <hash>\n    Expected canonical scenario SHA-256 from show-coverage --mapping-json; required with --test-mapping-index\n",
    },
    HelpContract {
        command: "generate-coverage",
        original: include_str!("../fixtures/help/generate-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/generate-coverage.txt"),
        hash: "67fcfe7251948c87a3cfc1a5c65550bb1e269bf9beda7ec1a02cf945b9a16d64",
        additions: "  --feature-name <name>\n    Generate only the exact selected feature; return a JSON plan, preserve lineage, and block unsafe removals or invalid metadata. With --dry-run, preview the same plan without writes.\n",
    },
    HelpContract {
        command: "show-coverage",
        original: include_str!("../fixtures/help/show-coverage.txt"),
        expected: include_str!("../fixtures/help/refac020/show-coverage.txt"),
        hash: "95cb036ddbe8fdb0c3778e23013457e8fee68c8a38d055b2b43f8086a09d5f1c",
        additions: "  --mapping-json\n    Read ordered mapping indices and canonical scenario hashes as JSON; requires a feature name and leaves --format json unchanged\n",
    },
];

pub fn assert_originals() -> TestResult {
    for contract in CONTRACTS {
        assert_eq!(hex::encode(Sha256::digest(contract.original.as_bytes())), contract.hash);
        let boundary = "\nCOMMON PATTERNS\n";
        assert_eq!(contract.original.matches(boundary).count(), 1);
        let expected = contract.original.replacen(boundary,
            &format!("{}\nCOMMON PATTERNS\n", contract.additions), 1);
        assert_eq!(contract.expected, expected, "{} fixture changed beyond its explicit approved option block", contract.command);
    }
    Ok(())
}
