#[allow(missing_docs)]
#[macro_export]
macro_rules! testgen_cmma_matmul {
    () => {
        #[test]
        pub fn test_matmul_cmma_one_cube() {
            tests::matmul_tests::test_matmul_cmma_one_cube::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_16_16() {
            tests::matmul_tests::test_matmul_cmma_16_16::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_32_16() {
            tests::matmul_tests::test_matmul_cmma_32_16::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_32_32() {
            tests::matmul_tests::test_matmul_cmma_32_32::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_64_16() {
            tests::matmul_tests::test_matmul_cmma_64_16::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_64_32() {
            tests::matmul_tests::test_matmul_cmma_64_32::<TestRuntime>(&Default::default())
        }

        #[test]
        #[ignore] // Activate only for f16
        pub fn test_matmul_cmma_64_64() {
            tests::matmul_tests::test_matmul_cmma_64_64::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_128_16() {
            tests::matmul_tests::test_matmul_cmma_128_16::<TestRuntime>(&Default::default())
        }

        #[test]
        #[ignore] // Activate only for f16
        pub fn test_matmul_cmma_128_32() {
            tests::matmul_tests::test_matmul_cmma_128_32::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_several_cubes() {
            tests::matmul_tests::test_matmul_cmma_several_cubes::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_with_check_bounds() {
            tests::matmul_tests::test_matmul_cmma_with_check_bounds::<TestRuntime>(
                &Default::default(),
            )
        }

        #[test]
        pub fn test_matmul_cmma_with_batches() {
            tests::matmul_tests::test_matmul_cmma_with_batches::<TestRuntime>(&Default::default())
        }

        #[test]
        pub fn test_matmul_cmma_unvectorizable_shapes() {
            tests::matmul_tests::test_matmul_cmma_unvectorizable_shapes::<TestRuntime>(
                &Default::default(),
            )
        }

        #[test]
        pub fn test_matmul_cmma_vec2_shapes() {
            tests::matmul_tests::test_matmul_cmma_vec2_shapes::<TestRuntime>(&Default::default())
        }
    };
}
