use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{build_g1_point, is_infinity};
use common::{parse_precompile_json, PrecompileTestCase};

struct EcAddTestCase {
    name: String,
    p1: [u8; 64],
    p2: [u8; 64],
    expected: [u8; 64],
}

fn parse_ecadd_test(test: &PrecompileTestCase) -> EcAddTestCase {
    let mut input = test.input.clone();
    input.resize(128, 0);

    let mut p1 = [0u8; 64];
    let mut p2 = [0u8; 64];
    p1.copy_from_slice(&input[..64]);
    p2.copy_from_slice(&input[64..128]);

    let bytes = test.expected.unwrap_success();
    let mut expected = [0u8; 64];
    let len = bytes.len().min(64);
    expected[..len].copy_from_slice(&bytes[..len]);

    EcAddTestCase {
        name: test.name.clone(),
        p1,
        p2,
        expected,
    }
}

pub fn ecadd_tests(crypto: &CustomEvmCrypto) {
    // 1] 0 + 0 = 0 (infinity + infinity = infinity)
    let p1 = build_g1_point("0", "0");
    let p2 = build_g1_point("0", "0");
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 1 should succeed");
    assert!(is_infinity(&result), "Test 1: 0 + 0 should be 0");

    // 2] 0 + P = P (infinity + P = P)
    let p1 = build_g1_point("0", "0");
    let p2 = build_g1_point("1", "2"); // Generator point G
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 2 should succeed");
    let expected = build_g1_point("1", "2");
    assert_eq!(result, expected, "Test 2: 0 + P should be P");

    // 3] P + 0 = P (P + infinity = P)
    let p1 = build_g1_point("1", "2"); // Generator point G
    let p2 = build_g1_point("0", "0");
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 3 should succeed");
    let expected = build_g1_point("1", "2");
    assert_eq!(result, expected, "Test 3: P + 0 should be P");

    // 4a] P1.x not in range (x >= P) - should fail
    let p1 = build_g1_point(
        "21888242871839275222246405745257275088696311157297823662689037894645226208584",
        "2",
    );
    let p2 = build_g1_point("3", "3");
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 4a: P1.x out of range should fail");

    // 4b] P1.y not in range (y >= P) - should fail
    let p1 = build_g1_point(
        "1",
        "21888242871839275222246405745257275088696311157297823662689037894645226208585",
    );
    let p2 = build_g1_point("3", "3");
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 4b: P1.y out of range should fail");

    // 5a] P2.x not in range (x >= P) - should fail
    let p1 = build_g1_point("1", "2");
    let p2 = build_g1_point(
        "21888242871839275222246405745257275088696311157297823662689037894645226208583",
        "0",
    );
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 5a: P2.x out of range should fail");

    // 5b] P2.y not in range (y >= P) - should fail
    let p1 = build_g1_point("1", "2");
    let p2 = build_g1_point(
        "0",
        "21888242871839275222246405745257275088696311157297823662689037894645226208583",
    );
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 5b: P2.y out of range should fail");

    // 6a] P1 not on curve (1, 0) - should fail
    let p1 = build_g1_point("1", "0");
    let p2 = build_g1_point("0", "0");
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 6a: P1 not on curve should fail");

    // 6b] P1 not on curve with valid P2 - should fail
    let p1 = build_g1_point("1", "0");
    let p2 = build_g1_point("1", "2");
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 6b: P1 not on curve should fail");

    // 7a] P2 not on curve (1, 0) with infinity P1 - should fail
    let p1 = build_g1_point("0", "0");
    let p2 = build_g1_point("1", "0");
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 7a: P2 not on curve should fail");

    // 7b] P2 not on curve with valid P1 - should fail
    let p1 = build_g1_point("1", "2");
    let p2 = build_g1_point("1", "0");
    let result = crypto.bn254_g1_add(&p1, &p2);
    assert!(result.is_err(), "Test 7b: P2 not on curve should fail");

    // 8] P + (-P) = 0 (point + negation = infinity)
    let p1 = build_g1_point(
        "10744596414106452074759370245733544594153395043370666422502510773307029471145",
        "848677436511517736191562425154572367705380862894644942948681172815252343932",
    );
    let p2 = build_g1_point(
        "10744596414106452074759370245733544594153395043370666422502510773307029471145",
        "21039565435327757486054843320102702720990930294403178719740356721829973864651",
    );
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 8 should succeed");
    assert!(is_infinity(&result), "Test 8: P + (-P) should be 0");

    // 9a] P + Q when P != Q (regular addition)
    let p1 = build_g1_point(
        "2893332206675025542079383054128180540025417352513932043566889211329192179032",
        "6530629491743359417280396166892081514007566149119717903717756741482263401518",
    );
    let p2 = build_g1_point(
        "15490799329273967747501973647822742581714860109251269127154113506193693607878",
        "4229358293223510599397432508631487048670295788986070026939193461742686527076",
    );
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 9a should succeed");
    let expected = build_g1_point(
        "13154776318592227270778558029295227935378730842313609923118896637591559850250",
        "11035980320923476543935377623718958678920911311849399323950347759358969041431",
    );
    assert_eq!(result, expected, "Test 9a: P + Q failed");

    // 9b] Another P + Q test
    let p1 = build_g1_point(
        "1745860766704548035074878643814414425056208216948549237180537806484993001172",
        "10428992577810537311515619307712828512800028181521723820412159824785899508051",
    );
    let p2 = build_g1_point(
        "10744596414106452074759370245733544594153395043370666422502510773307029471145",
        "848677436511517736191562425154572367705380862894644942948681172815252343932",
    );
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 9b should succeed");
    let expected = build_g1_point(
        "20109137777308224484751705964830245061785572657602899297228633767392913518415",
        "14499175368639637950478596677291617168262069295802020711454610174461584835979",
    );
    assert_eq!(result, expected, "Test 9b: P + Q failed");

    // 10a] P + P (point doubling)
    let p1 = build_g1_point(
        "2893332206675025542079383054128180540025417352513932043566889211329192179032",
        "6530629491743359417280396166892081514007566149119717903717756741482263401518",
    );
    let p2 = build_g1_point(
        "2893332206675025542079383054128180540025417352513932043566889211329192179032",
        "6530629491743359417280396166892081514007566149119717903717756741482263401518",
    );
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 10a should succeed");
    let expected = build_g1_point(
        "11220622501868821308995844886766009822833441579384302982823096531245924405698",
        "2355690023525969090855462437460037724073976772193253577110863269987724684477",
    );
    assert_eq!(result, expected, "Test 10a: P + P (doubling) failed");

    // 10b] Another doubling test
    let p1 = build_g1_point(
        "15490799329273967747501973647822742581714860109251269127154113506193693607878",
        "4229358293223510599397432508631487048670295788986070026939193461742686527076",
    );
    let p2 = build_g1_point(
        "15490799329273967747501973647822742581714860109251269127154113506193693607878",
        "4229358293223510599397432508631487048670295788986070026939193461742686527076",
    );
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 10b should succeed");
    let expected = build_g1_point(
        "14301632400969957113316344359548233118734763289927867040319376723985850943059",
        "19259402839901377893267670172732143592044261932601111690978918426524987173751",
    );
    assert_eq!(result, expected, "Test 10b: P + P (doubling) failed");

    // 10c] Another doubling test
    let p1 = build_g1_point(
        "1745860766704548035074878643814414425056208216948549237180537806484993001172",
        "10428992577810537311515619307712828512800028181521723820412159824785899508051",
    );
    let p2 = build_g1_point(
        "1745860766704548035074878643814414425056208216948549237180537806484993001172",
        "10428992577810537311515619307712828512800028181521723820412159824785899508051",
    );
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 10c should succeed");
    let expected = build_g1_point(
        "7635241416710394435863784018619353890364763495262225661273147225960091861733",
        "21716464559528323959695889215160185865818678200951896286120725092340748527691",
    );
    assert_eq!(result, expected, "Test 10c: P + P (doubling) failed");

    // 10d] Another doubling test
    let p1 = build_g1_point(
        "10744596414106452074759370245733544594153395043370666422502510773307029471145",
        "848677436511517736191562425154572367705380862894644942948681172815252343932",
    );
    let p2 = build_g1_point(
        "10744596414106452074759370245733544594153395043370666422502510773307029471145",
        "848677436511517736191562425154572367705380862894644942948681172815252343932",
    );
    let result = crypto
        .bn254_g1_add(&p1, &p2)
        .expect("Test 10d should succeed");
    let expected = build_g1_point(
        "4444740815889402603535294170722302758225367627362056425101568584910268024244",
        "10537263096529483164618820017164668921386457028564663708352735080900270541420",
    );
    assert_eq!(result, expected, "Test 10d: P + P (doubling) failed");

    // Geth test vectors
    for test in &parse_precompile_json(include_str!("../testdata/bn256Add.json")) {
        let t = parse_ecadd_test(test);
        let result = crypto.bn254_g1_add(&t.p1, &t.p2);
        assert!(result.is_ok(), "bn254Add {} should succeed", t.name);
        assert_eq!(result.unwrap(), t.expected, "bn254Add {} mismatch", t.name);
    }

    println!("All EcAdd tests passed!");
}
