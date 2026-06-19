use guest_reth::CustomEvmCrypto;
use revm::precompile::Crypto;

use super::common::{build_g1_point, decimal_to_32, is_infinity};
use common::{parse_precompile_json, PrecompileTestCase};

struct EcMulTestCase {
    name: String,
    point: [u8; 64],
    scalar: [u8; 32],
    expected: [u8; 64],
}

fn parse_ecmul_test(test: &PrecompileTestCase) -> EcMulTestCase {
    let mut input = test.input.clone();
    input.resize(96, 0);

    let mut point = [0u8; 64];
    let mut scalar = [0u8; 32];
    point.copy_from_slice(&input[..64]);
    scalar.copy_from_slice(&input[64..96]);

    let bytes = test.expected.unwrap_success();
    let mut expected = [0u8; 64];
    let len = bytes.len().min(64);
    expected[..len].copy_from_slice(&bytes[..len]);

    EcMulTestCase {
        name: test.name.clone(),
        point,
        scalar,
        expected,
    }
}

pub fn ecmul_tests(crypto: &CustomEvmCrypto) {
    // 1] 0·O = O (zero scalar times infinity = infinity)
    let point = build_g1_point("0", "0");
    let scalar = decimal_to_32("0");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 1 should succeed");
    assert!(is_infinity(&result), "Test 1: 0·O should be O");

    // 2] k·O = O (any scalar times infinity = infinity)
    let point = build_g1_point("0", "0");
    let scalar = decimal_to_32("5");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 2 should succeed");
    assert!(is_infinity(&result), "Test 2: k·O should be O");

    // 3] 0·P = O (zero scalar times any point = infinity)
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32("0");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 3 should succeed");
    assert!(is_infinity(&result), "Test 3: 0·P should be O");

    // 4a] P.x not in range (x >= P) - should fail
    let point = build_g1_point(
        "21888242871839275222246405745257275088696311157297823662689037894645226208584",
        "2",
    );
    let scalar = decimal_to_32("0");
    let result = crypto.bn254_g1_mul(&point, &scalar);
    assert!(result.is_err(), "Test 4a: P.x out of range should fail");

    // 4b] P.y not in range (y >= P) - should fail
    let point = build_g1_point(
        "1",
        "21888242871839275222246405745257275088696311157297823662689037894645226208585",
    );
    let scalar = decimal_to_32("0");
    let result = crypto.bn254_g1_mul(&point, &scalar);
    assert!(result.is_err(), "Test 4b: P.y out of range should fail");

    // 5a] P not on curve with k=0 - should fail
    let point = build_g1_point("1", "0");
    let scalar = decimal_to_32("0");
    let result = crypto.bn254_g1_mul(&point, &scalar);
    assert!(result.is_err(), "Test 5a: P not on curve should fail");

    // 5b] P not on curve with k=65 - should fail
    let point = build_g1_point("1", "0");
    let scalar = decimal_to_32("65");
    let result = crypto.bn254_g1_mul(&point, &scalar);
    assert!(result.is_err(), "Test 5b: P not on curve should fail");

    // 6a] 1·G = G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32("1");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6a should succeed");
    let expected = build_g1_point("1", "2");
    assert_eq!(result, expected, "Test 6a: 1·G should be G");

    // 6b] 2·G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32("2");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6b should succeed");
    let expected = build_g1_point(
        "1368015179489954701390400359078579693043519447331113978918064868415326638035",
        "9918110051302171585080402603319702774565515993150576347155970296011118125764",
    );
    assert_eq!(result, expected, "Test 6b: 2·G failed");

    // 6c] 65·G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32("65");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6c should succeed");
    let expected = build_g1_point(
        "21184532036463169063041779836861514142873086093180850953095098556309204188255",
        "16870949628445799017882714788639508275834535486794531840392367353784571921174",
    );
    assert_eq!(result, expected, "Test 6c: 65·G failed");

    // 6d] 10000000089·G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32("10000000089");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6d should succeed");
    let expected = build_g1_point(
        "4768044760451824005417871472283223457728569810854115125480649095031772328870",
        "21389337952468851259287213083493638952853622949895525580347877121675081015727",
    );
    assert_eq!(result, expected, "Test 6d: 10000000089·G failed");

    // 6e] 57·P (different base point)
    let point = build_g1_point(
        "1745860766704548035074878643814414425056208216948549237180537806484993001172",
        "10428992577810537311515619307712828512800028181521723820412159824785899508051",
    );
    let scalar = decimal_to_32("57");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6e should succeed");
    let expected = build_g1_point(
        "21092868577100313210583214784627729175513062432513303686654820611840644382013",
        "10293123368529248350591404721829100625076077203595282162629899903703630633665",
    );
    assert_eq!(result, expected, "Test 6e: 57·P failed");

    // 6f] 123456789·P
    let point = build_g1_point(
        "1745860766704548035074878643814414425056208216948549237180537806484993001172",
        "10428992577810537311515619307712828512800028181521723820412159824785899508051",
    );
    let scalar = decimal_to_32("123456789");
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6f should succeed");
    let expected = build_g1_point(
        "9551410454255481932113938269904288675272239827491596157984458647610565008967",
        "17781856861347070862134441477208204792978952663354273425763774350233183876915",
    );
    assert_eq!(result, expected, "Test 6f: 123456789·P failed");

    // 6g] r·G = O (scalar equals group order)
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32(
        "21888242871839275222246405745257275088548364400416034343698204186575808495617",
    );
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6g should succeed");
    assert!(is_infinity(&result), "Test 6g: r·G should be O");

    // 6h] (r+1)·G = G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32(
        "21888242871839275222246405745257275088548364400416034343698204186575808495618",
    );
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6h should succeed");
    let expected = build_g1_point("1", "2");
    assert_eq!(result, expected, "Test 6h: (r+1)·G should be G");

    // 6i] P (field modulus) · G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32(
        "21888242871839275222246405745257275088696311157297823662689037894645226208583",
    );
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6i should succeed");
    let expected = build_g1_point(
        "7793429943220682609834519115512946233910458086191548249060013461061457526887",
        "16460968250425543446028981775631045522280113359306664586749259656855967130574",
    );
    assert_eq!(result, expected, "Test 6i: P·G failed");

    // 6j] (P+1) · G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32(
        "21888242871839275222246405745257275088696311157297823662689037894645226208584",
    );
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6j should succeed");
    let expected = build_g1_point(
        "15886422571275617715400903250697722692198979607302343556925904858625057687404",
        "9788557113822741943783365060165103517008620829146475047263378292709661309554",
    );
    assert_eq!(result, expected, "Test 6j: (P+1)·G failed");

    // 6k] Large scalar (2^256 - 1) · G
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32(
        "115792089237316195423570985008687907853269984665640564039457584007913129639935",
    );
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 6k should succeed");
    let expected = build_g1_point(
        "21415159568991615317144600033915305503576371596506956373206836402282692989778",
        "8573070896319864868535933562264623076420652926303237982078693068147657243287",
    );
    assert_eq!(result, expected, "Test 6k: (2^256-1)·G failed");

    // 7] Worst case scenario: scalar with highest Hamming weight < r (2^253 - 1)
    let point = build_g1_point("1", "2");
    let scalar = decimal_to_32(
        "14474011154664524427946373126085988481658748083205070504932198000989141204991",
    );
    let result = crypto
        .bn254_g1_mul(&point, &scalar)
        .expect("Test 7 should succeed");
    let expected = build_g1_point(
        "3739418567393436576913511739065691570763034865122368432616000129799288055432",
        "18298856760603404171434473181920219106007178146585940397845192637485681860518",
    );
    assert_eq!(
        result, expected,
        "Test 7: worst case scalar multiplication failed"
    );

    // Geth test vectors
    for test in &parse_precompile_json(include_str!("../testdata/bn256ScalarMul.json")) {
        let t = parse_ecmul_test(test);
        let result = crypto.bn254_g1_mul(&t.point, &t.scalar);
        assert!(result.is_ok(), "bn254ScalarMul {} should succeed", t.name);
        assert_eq!(
            result.unwrap(),
            t.expected,
            "bn254ScalarMul {} mismatch",
            t.name
        );
    }

    println!("All EcMul tests passed!");
}
