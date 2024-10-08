use std::ops::Range;

use cubecl_core as cubecl;
use cubecl_core::prelude::*;

use crate::matmul::cmma::base::{Dimensions, Ids, Offsets, RuntimeCmmaInfo};
use crate::matmul::cmma::config::{CmmaConfig, WriteOutStrategy};
use crate::matmul::tests::test_utils::{assert_equals_range, create_empty};
use crate::matmul::{
    cmma::{config::ComptimeCmmaInfo, load_shared_memory::*},
    tests::test_utils::range_tensor,
};

use super::base::{DimsTestCase, B_K, B_MN};

#[cube(launch_unchecked)]
fn load_lhs_test<F: Float>(
    lhs_tensor: &Tensor<F>,
    lhs_sm_arr: &mut Array<F>,
    k_offset: u32,
    m: u32,
    k: u32,
    n: u32,
    #[comptime] config: ComptimeCmmaInfo,
) {
    let block_size_m = config.block_size_m;
    let block_size_k = config.block_size_k;
    let sm_size = block_size_k * block_size_m;

    let mut lhs_sm = SharedMemory::<F>::new(sm_size);
    for i in 0..sm_size {
        lhs_sm[i] = F::new(0.);
    }

    let offsets = Offsets {
        batch_lhs: 0,
        batch_rhs: 0,
        batch_out: 0,
        cube_row: 0,
        cube_col: 0,
    };
    let dims = Dimensions { m, k, n };
    let ids = Ids {
        coop: UNIT_POS_Y,
        lane: UNIT_POS_X,
    };
    let runtime_info = RuntimeCmmaInfo { offsets, dims, ids };

    load_lhs(lhs_tensor, &mut lhs_sm, 2, k_offset, runtime_info, config);

    for i in 0..sm_size {
        lhs_sm_arr[i] = lhs_sm[i];
    }
}

#[cube(launch_unchecked)]
fn load_rhs_test<F: Float>(
    rhs_tensor: &Tensor<F>,
    rhs_sm_arr: &mut Array<F>,
    k_offset: u32,
    m: u32,
    k: u32,
    n: u32,
    #[comptime] config: ComptimeCmmaInfo,
) {
    let block_size_k = config.block_size_k;
    let block_size_n = config.block_size_n;
    let sm_size = block_size_k * block_size_n;
    let mut rhs_sm = SharedMemory::<F>::new(sm_size);

    for i in 0..sm_size {
        rhs_sm[i] = F::new(0.);
    }

    let offsets = Offsets {
        batch_lhs: 0,
        batch_rhs: 0,
        batch_out: 0,
        cube_row: 0,
        cube_col: 0,
    };
    let dims = Dimensions { m, k, n };
    let ids = Ids {
        coop: UNIT_POS_Y,
        lane: UNIT_POS_X,
    };
    let runtime_info = RuntimeCmmaInfo { offsets, dims, ids };

    load_rhs(rhs_tensor, &mut rhs_sm, 2, k_offset, runtime_info, config);

    for i in 0..sm_size {
        rhs_sm_arr[i] = rhs_sm[i];
    }
}

enum InputTensor {
    Lhs,
    Rhs,
}

fn load_shared_memory_test_case<R: Runtime>(
    input: InputTensor,
    dims: DimsTestCase,
    k_offset: usize,
    config: CmmaConfig,
    expected: &[f32],
    device: &R::Device,
    range: Range<usize>,
) {
    let client = R::client(device);

    for vectorization in [1, 2, 4] {
        let smem = create_empty::<R>(&client, config.b_k, config.b_mn);
        let smem_size = config.b_k * config.b_mn;

        match input {
            InputTensor::Lhs => {
                let tensor = range_tensor::<R>(&client, dims.m, dims.k);

                unsafe {
                    load_lhs_test::launch_unchecked::<f32, R>(
                        &R::client(device),
                        config.cube_count::<R>(&[dims.m, dims.n]),
                        config.cube_dim(),
                        TensorArg::from_raw_parts(
                            &tensor.handle,
                            &tensor.strides,
                            &tensor.shape,
                            vectorization,
                        ),
                        ArrayArg::from_raw_parts(&smem, smem_size, 1),
                        ScalarArg::new(k_offset as u32),
                        ScalarArg::new(dims.m as u32),
                        ScalarArg::new(dims.k as u32),
                        ScalarArg::new(dims.n as u32),
                        config.comptime_info(dims.m, dims.k, dims.n),
                    );
                };
            }
            InputTensor::Rhs => {
                let tensor = range_tensor::<R>(&client, dims.k, dims.n);

                unsafe {
                    load_rhs_test::launch_unchecked::<f32, R>(
                        &R::client(device),
                        config.cube_count::<R>(&[dims.m, dims.n]),
                        config.cube_dim(),
                        TensorArg::from_raw_parts(
                            &tensor.handle,
                            &tensor.strides,
                            &tensor.shape,
                            vectorization,
                        ),
                        ArrayArg::from_raw_parts(&smem, smem_size, 1),
                        ScalarArg::new(k_offset as u32),
                        ScalarArg::new(dims.m as u32),
                        ScalarArg::new(dims.k as u32),
                        ScalarArg::new(dims.n as u32),
                        config.comptime_info(dims.m, dims.k, dims.n),
                    );
                };
            }
        }

        assert_equals_range::<R>(&client, smem, expected, range.clone());
    }
}

/// Exported test
pub fn load_shared_memory_lhs_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Lhs,
        DimsTestCase {
            m: 64,
            k: 32,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0,
            46.0, 47.0, 64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 71.0, 72.0, 73.0, 74.0, 75.0,
            76.0, 77.0, 78.0, 79.0, 96.0, 97.0, 98.0, 99.0, 100.0, 101.0, 102.0, 103.0, 104.0,
            105.0, 106.0, 107.0, 108.0, 109.0, 110.0, 111.0, 128.0, 129.0, 130.0, 131.0, 132.0,
            133.0, 134.0, 135.0, 136.0, 137.0, 138.0, 139.0, 140.0, 141.0, 142.0, 143.0, 160.0,
            161.0, 162.0, 163.0, 164.0, 165.0, 166.0, 167.0, 168.0, 169.0, 170.0, 171.0, 172.0,
            173.0, 174.0, 175.0, 192.0, 193.0, 194.0, 195.0, 196.0, 197.0, 198.0, 199.0, 200.0,
            201.0, 202.0, 203.0, 204.0, 205.0, 206.0, 207.0, 224.0, 225.0, 226.0, 227.0, 228.0,
            229.0, 230.0, 231.0, 232.0, 233.0, 234.0, 235.0, 236.0, 237.0, 238.0, 239.0, 256.0,
            257.0, 258.0, 259.0, 260.0, 261.0, 262.0, 263.0, 264.0, 265.0, 266.0, 267.0, 268.0,
            269.0, 270.0, 271.0, 288.0, 289.0, 290.0, 291.0, 292.0, 293.0, 294.0, 295.0, 296.0,
            297.0, 298.0, 299.0, 300.0, 301.0, 302.0, 303.0, 320.0, 321.0, 322.0, 323.0, 324.0,
            325.0, 326.0, 327.0, 328.0, 329.0, 330.0, 331.0, 332.0, 333.0, 334.0, 335.0, 352.0,
            353.0, 354.0, 355.0, 356.0, 357.0, 358.0, 359.0, 360.0, 361.0, 362.0, 363.0, 364.0,
            365.0, 366.0, 367.0, 384.0, 385.0, 386.0, 387.0, 388.0, 389.0, 390.0, 391.0, 392.0,
            393.0, 394.0, 395.0, 396.0, 397.0, 398.0, 399.0, 416.0, 417.0, 418.0, 419.0, 420.0,
            421.0, 422.0, 423.0, 424.0, 425.0, 426.0, 427.0, 428.0, 429.0, 430.0, 431.0, 448.0,
            449.0, 450.0, 451.0, 452.0, 453.0, 454.0, 455.0, 456.0, 457.0, 458.0, 459.0, 460.0,
            461.0, 462.0, 463.0, 480.0, 481.0, 482.0, 483.0, 484.0, 485.0, 486.0, 487.0, 488.0,
            489.0, 490.0, 491.0, 492.0, 493.0, 494.0, 495.0,
        ],
        device,
        0..256,
    );
}

/// Exported test
pub fn load_shared_memory_rhs_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Rhs,
        DimsTestCase {
            m: 64,
            k: 32,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 71.0, 72.0, 73.0, 74.0, 75.0, 76.0, 77.0,
            78.0, 79.0, 128.0, 129.0, 130.0, 131.0, 132.0, 133.0, 134.0, 135.0, 136.0, 137.0,
            138.0, 139.0, 140.0, 141.0, 142.0, 143.0, 192.0, 193.0, 194.0, 195.0, 196.0, 197.0,
            198.0, 199.0, 200.0, 201.0, 202.0, 203.0, 204.0, 205.0, 206.0, 207.0, 256.0, 257.0,
            258.0, 259.0, 260.0, 261.0, 262.0, 263.0, 264.0, 265.0, 266.0, 267.0, 268.0, 269.0,
            270.0, 271.0, 320.0, 321.0, 322.0, 323.0, 324.0, 325.0, 326.0, 327.0, 328.0, 329.0,
            330.0, 331.0, 332.0, 333.0, 334.0, 335.0, 384.0, 385.0, 386.0, 387.0, 388.0, 389.0,
            390.0, 391.0, 392.0, 393.0, 394.0, 395.0, 396.0, 397.0, 398.0, 399.0, 448.0, 449.0,
            450.0, 451.0, 452.0, 453.0, 454.0, 455.0, 456.0, 457.0, 458.0, 459.0, 460.0, 461.0,
            462.0, 463.0, 512.0, 513.0, 514.0, 515.0, 516.0, 517.0, 518.0, 519.0, 520.0, 521.0,
            522.0, 523.0, 524.0, 525.0, 526.0, 527.0, 576.0, 577.0, 578.0, 579.0, 580.0, 581.0,
            582.0, 583.0, 584.0, 585.0, 586.0, 587.0, 588.0, 589.0, 590.0, 591.0, 640.0, 641.0,
            642.0, 643.0, 644.0, 645.0, 646.0, 647.0, 648.0, 649.0, 650.0, 651.0, 652.0, 653.0,
            654.0, 655.0, 704.0, 705.0, 706.0, 707.0, 708.0, 709.0, 710.0, 711.0, 712.0, 713.0,
            714.0, 715.0, 716.0, 717.0, 718.0, 719.0, 768.0, 769.0, 770.0, 771.0, 772.0, 773.0,
            774.0, 775.0, 776.0, 777.0, 778.0, 779.0, 780.0, 781.0, 782.0, 783.0, 832.0, 833.0,
            834.0, 835.0, 836.0, 837.0, 838.0, 839.0, 840.0, 841.0, 842.0, 843.0, 844.0, 845.0,
            846.0, 847.0, 896.0, 897.0, 898.0, 899.0, 900.0, 901.0, 902.0, 903.0, 904.0, 905.0,
            906.0, 907.0, 908.0, 909.0, 910.0, 911.0, 960.0, 961.0, 962.0, 963.0, 964.0, 965.0,
            966.0, 967.0, 968.0, 969.0, 970.0, 971.0, 972.0, 973.0, 974.0, 975.0,
        ],
        device,
        0..256,
    );
}

/// Exported test
pub fn load_shared_memory_lhs_vertical_out_of_bound_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Lhs,
        DimsTestCase {
            m: 12,
            k: 64,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 71.0, 72.0, 73.0, 74.0, 75.0, 76.0, 77.0,
            78.0, 79.0, 128.0, 129.0, 130.0, 131.0, 132.0, 133.0, 134.0, 135.0, 136.0, 137.0,
            138.0, 139.0, 140.0, 141.0, 142.0, 143.0, 192.0, 193.0, 194.0, 195.0, 196.0, 197.0,
            198.0, 199.0, 200.0, 201.0, 202.0, 203.0, 204.0, 205.0, 206.0, 207.0, 256.0, 257.0,
            258.0, 259.0, 260.0, 261.0, 262.0, 263.0, 264.0, 265.0, 266.0, 267.0, 268.0, 269.0,
            270.0, 271.0, 320.0, 321.0, 322.0, 323.0, 324.0, 325.0, 326.0, 327.0, 328.0, 329.0,
            330.0, 331.0, 332.0, 333.0, 334.0, 335.0, 384.0, 385.0, 386.0, 387.0, 388.0, 389.0,
            390.0, 391.0, 392.0, 393.0, 394.0, 395.0, 396.0, 397.0, 398.0, 399.0, 448.0, 449.0,
            450.0, 451.0, 452.0, 453.0, 454.0, 455.0, 456.0, 457.0, 458.0, 459.0, 460.0, 461.0,
            462.0, 463.0, 512.0, 513.0, 514.0, 515.0, 516.0, 517.0, 518.0, 519.0, 520.0, 521.0,
            522.0, 523.0, 524.0, 525.0, 526.0, 527.0, 576.0, 577.0, 578.0, 579.0, 580.0, 581.0,
            582.0, 583.0, 584.0, 585.0, 586.0, 587.0, 588.0, 589.0, 590.0, 591.0, 640.0, 641.0,
            642.0, 643.0, 644.0, 645.0, 646.0, 647.0, 648.0, 649.0, 650.0, 651.0, 652.0, 653.0,
            654.0, 655.0, 704.0, 705.0, 706.0, 707.0, 708.0, 709.0, 710.0, 711.0, 712.0, 713.0,
            714.0, 715.0, 716.0, 717.0, 718.0, 719.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
        ],
        device,
        0..256,
    );
}

/// Exported test
pub fn load_shared_memory_lhs_horizontal_out_of_bound_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Lhs,
        DimsTestCase {
            m: 64,
            k: 12,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 0.0, 0.0, 0.0, 0.0, 12.0,
            13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 0.0, 0.0, 0.0, 0.0,
            24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 0.0, 0.0, 0.0,
            0.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 47.0, 0.0, 0.0,
            0.0, 0.0, 48.0, 49.0, 50.0, 51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0, 58.0, 59.0, 0.0,
            0.0, 0.0, 0.0, 60.0, 61.0, 62.0, 63.0, 64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 71.0,
            0.0, 0.0, 0.0, 0.0, 72.0, 73.0, 74.0, 75.0, 76.0, 77.0, 78.0, 79.0, 80.0, 81.0, 82.0,
            83.0, 0.0, 0.0, 0.0, 0.0, 84.0, 85.0, 86.0, 87.0, 88.0, 89.0, 90.0, 91.0, 92.0, 93.0,
            94.0, 95.0, 0.0, 0.0, 0.0, 0.0, 96.0, 97.0, 98.0, 99.0, 100.0, 101.0, 102.0, 103.0,
            104.0, 105.0, 106.0, 107.0, 0.0, 0.0, 0.0, 0.0, 108.0, 109.0, 110.0, 111.0, 112.0,
            113.0, 114.0, 115.0, 116.0, 117.0, 118.0, 119.0, 0.0, 0.0, 0.0, 0.0, 120.0, 121.0,
            122.0, 123.0, 124.0, 125.0, 126.0, 127.0, 128.0, 129.0, 130.0, 131.0, 0.0, 0.0, 0.0,
            0.0, 132.0, 133.0, 134.0, 135.0, 136.0, 137.0, 138.0, 139.0, 140.0, 141.0, 142.0,
            143.0, 0.0, 0.0, 0.0, 0.0, 144.0, 145.0, 146.0, 147.0, 148.0, 149.0, 150.0, 151.0,
            152.0, 153.0, 154.0, 155.0, 0.0, 0.0, 0.0, 0.0, 156.0, 157.0, 158.0, 159.0, 160.0,
            161.0, 162.0, 163.0, 164.0, 165.0, 166.0, 167.0, 0.0, 0.0, 0.0, 0.0, 168.0, 169.0,
            170.0, 171.0, 172.0, 173.0, 174.0, 175.0, 176.0, 177.0, 178.0, 179.0, 0.0, 0.0, 0.0,
            0.0, 180.0, 181.0, 182.0, 183.0, 184.0, 185.0, 186.0, 187.0, 188.0, 189.0, 190.0,
            191.0, 0.0, 0.0, 0.0, 0.0,
        ],
        device,
        0..256,
    );
}

/// Exported test
pub fn load_shared_memory_lhs_whole_out_of_bound_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Lhs,
        DimsTestCase {
            m: 12,
            k: 12,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 0.0, 0.0, 0.0, 0.0, 12.0,
            13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 0.0, 0.0, 0.0, 0.0,
            24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 0.0, 0.0, 0.0,
            0.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 47.0, 0.0, 0.0,
            0.0, 0.0, 48.0, 49.0, 50.0, 51.0, 52.0, 53.0, 54.0, 55.0, 56.0, 57.0, 58.0, 59.0, 0.0,
            0.0, 0.0, 0.0, 60.0, 61.0, 62.0, 63.0, 64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 71.0,
            0.0, 0.0, 0.0, 0.0, 72.0, 73.0, 74.0, 75.0, 76.0, 77.0, 78.0, 79.0, 80.0, 81.0, 82.0,
            83.0, 0.0, 0.0, 0.0, 0.0, 84.0, 85.0, 86.0, 87.0, 88.0, 89.0, 90.0, 91.0, 92.0, 93.0,
            94.0, 95.0, 0.0, 0.0, 0.0, 0.0, 96.0, 97.0, 98.0, 99.0, 100.0, 101.0, 102.0, 103.0,
            104.0, 105.0, 106.0, 107.0, 0.0, 0.0, 0.0, 0.0, 108.0, 109.0, 110.0, 111.0, 112.0,
            113.0, 114.0, 115.0, 116.0, 117.0, 118.0, 119.0, 0.0, 0.0, 0.0, 0.0, 120.0, 121.0,
            122.0, 123.0, 124.0, 125.0, 126.0, 127.0, 128.0, 129.0, 130.0, 131.0, 0.0, 0.0, 0.0,
            0.0, 132.0, 133.0, 134.0, 135.0, 136.0, 137.0, 138.0, 139.0, 140.0, 141.0, 142.0,
            143.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0,
        ],
        device,
        0..256,
    );
}

/// Exported test
pub fn load_shared_memory_lhs_second_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Lhs,
        DimsTestCase {
            m: 64,
            k: 64,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            16., 17., 18., 19., 20., 21., 22., 23., 24., 25., 26., 27., 28., 29., 30., 31., 80.,
            81., 82., 83., 84., 85., 86., 87., 88., 89., 90., 91., 92., 93., 94., 95., 144., 145.,
            146., 147., 148., 149., 150., 151., 152., 153., 154., 155., 156., 157., 158., 159.,
            208., 209., 210., 211., 212., 213., 214., 215., 216., 217., 218., 219., 220., 221.,
            222., 223., 272., 273., 274., 275., 276., 277., 278., 279., 280., 281., 282., 283.,
            284., 285., 286., 287., 336., 337., 338., 339., 340., 341., 342., 343., 344., 345.,
            346., 347., 348., 349., 350., 351., 400., 401., 402., 403., 404., 405., 406., 407.,
            408., 409., 410., 411., 412., 413., 414., 415., 464., 465., 466., 467., 468., 469.,
            470., 471., 472., 473., 474., 475., 476., 477., 478., 479., 528., 529., 530., 531.,
            532., 533., 534., 535., 536., 537., 538., 539., 540., 541., 542., 543., 592., 593.,
            594., 595., 596., 597., 598., 599., 600., 601., 602., 603., 604., 605., 606., 607.,
            656., 657., 658., 659., 660., 661., 662., 663., 664., 665., 666., 667., 668., 669.,
            670., 671., 720., 721., 722., 723., 724., 725., 726., 727., 728., 729., 730., 731.,
            732., 733., 734., 735., 784., 785., 786., 787., 788., 789., 790., 791., 792., 793.,
            794., 795., 796., 797., 798., 799., 848., 849., 850., 851., 852., 853., 854., 855.,
            856., 857., 858., 859., 860., 861., 862., 863., 912., 913., 914., 915., 916., 917.,
            918., 919., 920., 921., 922., 923., 924., 925., 926., 927., 976., 977., 978., 979.,
            980., 981., 982., 983., 984., 985., 986., 987., 988., 989., 990., 991.,
        ],
        device,
        256..512,
    );
}

/// Exported test
pub fn load_shared_memory_rhs_second_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Rhs,
        DimsTestCase {
            m: 64,
            k: 64,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            1024., 1025., 1026., 1027., 1028., 1029., 1030., 1031., 1032., 1033., 1034., 1035.,
            1036., 1037., 1038., 1039., 1088., 1089., 1090., 1091., 1092., 1093., 1094., 1095.,
            1096., 1097., 1098., 1099., 1100., 1101., 1102., 1103., 1152., 1153., 1154., 1155.,
            1156., 1157., 1158., 1159., 1160., 1161., 1162., 1163., 1164., 1165., 1166., 1167.,
            1216., 1217., 1218., 1219., 1220., 1221., 1222., 1223., 1224., 1225., 1226., 1227.,
            1228., 1229., 1230., 1231., 1280., 1281., 1282., 1283., 1284., 1285., 1286., 1287.,
            1288., 1289., 1290., 1291., 1292., 1293., 1294., 1295., 1344., 1345., 1346., 1347.,
            1348., 1349., 1350., 1351., 1352., 1353., 1354., 1355., 1356., 1357., 1358., 1359.,
            1408., 1409., 1410., 1411., 1412., 1413., 1414., 1415., 1416., 1417., 1418., 1419.,
            1420., 1421., 1422., 1423., 1472., 1473., 1474., 1475., 1476., 1477., 1478., 1479.,
            1480., 1481., 1482., 1483., 1484., 1485., 1486., 1487., 1536., 1537., 1538., 1539.,
            1540., 1541., 1542., 1543., 1544., 1545., 1546., 1547., 1548., 1549., 1550., 1551.,
            1600., 1601., 1602., 1603., 1604., 1605., 1606., 1607., 1608., 1609., 1610., 1611.,
            1612., 1613., 1614., 1615., 1664., 1665., 1666., 1667., 1668., 1669., 1670., 1671.,
            1672., 1673., 1674., 1675., 1676., 1677., 1678., 1679., 1728., 1729., 1730., 1731.,
            1732., 1733., 1734., 1735., 1736., 1737., 1738., 1739., 1740., 1741., 1742., 1743.,
            1792., 1793., 1794., 1795., 1796., 1797., 1798., 1799., 1800., 1801., 1802., 1803.,
            1804., 1805., 1806., 1807., 1856., 1857., 1858., 1859., 1860., 1861., 1862., 1863.,
            1864., 1865., 1866., 1867., 1868., 1869., 1870., 1871., 1920., 1921., 1922., 1923.,
            1924., 1925., 1926., 1927., 1928., 1929., 1930., 1931., 1932., 1933., 1934., 1935.,
            1984., 1985., 1986., 1987., 1988., 1989., 1990., 1991., 1992., 1993., 1994., 1995.,
            1996., 1997., 1998., 1999.,
        ],
        device,
        256..512,
    );
}

/// Exported test
pub fn load_shared_memory_lhs_third_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Lhs,
        DimsTestCase {
            m: 64,
            k: 64,
            n: 64,
        },
        0,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0,
            30.0, 31.0, 80.0, 81.0, 82.0, 83.0, 84.0, 85.0, 86.0, 87.0, 88.0, 89.0, 90.0, 91.0,
            92.0, 93.0, 94.0, 95.0, 144.0, 145.0, 146.0, 147.0, 148.0, 149.0, 150.0, 151.0, 152.0,
            153.0, 154.0, 155.0, 156.0, 157.0, 158.0, 159.0, 208.0, 209.0, 210.0, 211.0, 212.0,
            213.0, 214.0, 215.0, 216.0, 217.0, 218.0, 219.0, 220.0, 221.0, 222.0, 223.0, 272.0,
            273.0, 274.0, 275.0, 276.0, 277.0, 278.0, 279.0, 280.0, 281.0, 282.0, 283.0, 284.0,
            285.0, 286.0, 287.0, 336.0, 337.0, 338.0, 339.0, 340.0, 341.0, 342.0, 343.0, 344.0,
            345.0, 346.0, 347.0, 348.0, 349.0, 350.0, 351.0, 400.0, 401.0, 402.0, 403.0, 404.0,
            405.0, 406.0, 407.0, 408.0, 409.0, 410.0, 411.0, 412.0, 413.0, 414.0, 415.0, 464.0,
            465.0, 466.0, 467.0, 468.0, 469.0, 470.0, 471.0, 472.0, 473.0, 474.0, 475.0, 476.0,
            477.0, 478.0, 479.0, 528.0, 529.0, 530.0, 531.0, 532.0, 533.0, 534.0, 535.0, 536.0,
            537.0, 538.0, 539.0, 540.0, 541.0, 542.0, 543.0, 592.0, 593.0, 594.0, 595.0, 596.0,
            597.0, 598.0, 599.0, 600.0, 601.0, 602.0, 603.0, 604.0, 605.0, 606.0, 607.0, 656.0,
            657.0, 658.0, 659.0, 660.0, 661.0, 662.0, 663.0, 664.0, 665.0, 666.0, 667.0, 668.0,
            669.0, 670.0, 671.0, 720.0, 721.0, 722.0, 723.0, 724.0, 725.0, 726.0, 727.0, 728.0,
            729.0, 730.0, 731.0, 732.0, 733.0, 734.0, 735.0, 784.0, 785.0, 786.0, 787.0, 788.0,
            789.0, 790.0, 791.0, 792.0, 793.0, 794.0, 795.0, 796.0, 797.0, 798.0, 799.0, 848.0,
            849.0, 850.0, 851.0, 852.0, 853.0, 854.0, 855.0, 856.0, 857.0, 858.0, 859.0, 860.0,
            861.0, 862.0, 863.0, 912.0, 913.0, 914.0, 915.0, 916.0, 917.0, 918.0, 919.0, 920.0,
            921.0, 922.0, 923.0, 924.0, 925.0, 926.0, 927.0, 976.0, 977.0, 978.0, 979.0, 980.0,
            981.0, 982.0, 983.0, 984.0, 985.0, 986.0, 987.0, 988.0, 989.0, 990.0, 991.0,
        ],
        device,
        256..512,
    );
}

/// Exported test
pub fn load_shared_memory_rhs_third_warp_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Rhs,
        DimsTestCase {
            m: 64,
            k: 64,
            n: 64,
        },
        0,
        CmmaConfig::new(64, 32, false, WriteOutStrategy::LargeSmem),
        &[
            16., 17., 18., 19., 20., 21., 22., 23., 24., 25., 26., 27., 28., 29., 30., 31., 80.,
            81., 82., 83., 84., 85., 86., 87., 88., 89., 90., 91., 92., 93., 94., 95., 144., 145.,
            146., 147., 148., 149., 150., 151., 152., 153., 154., 155., 156., 157., 158., 159.,
            208., 209., 210., 211., 212., 213., 214., 215., 216., 217., 218., 219., 220., 221.,
            222., 223., 272., 273., 274., 275., 276., 277., 278., 279., 280., 281., 282., 283.,
            284., 285., 286., 287., 336., 337., 338., 339., 340., 341., 342., 343., 344., 345.,
            346., 347., 348., 349., 350., 351., 400., 401., 402., 403., 404., 405., 406., 407.,
            408., 409., 410., 411., 412., 413., 414., 415., 464., 465., 466., 467., 468., 469.,
            470., 471., 472., 473., 474., 475., 476., 477., 478., 479., 528., 529., 530., 531.,
            532., 533., 534., 535., 536., 537., 538., 539., 540., 541., 542., 543., 592., 593.,
            594., 595., 596., 597., 598., 599., 600., 601., 602., 603., 604., 605., 606., 607.,
            656., 657., 658., 659., 660., 661., 662., 663., 664., 665., 666., 667., 668., 669.,
            670., 671., 720., 721., 722., 723., 724., 725., 726., 727., 728., 729., 730., 731.,
            732., 733., 734., 735., 784., 785., 786., 787., 788., 789., 790., 791., 792., 793.,
            794., 795., 796., 797., 798., 799., 848., 849., 850., 851., 852., 853., 854., 855.,
            856., 857., 858., 859., 860., 861., 862., 863., 912., 913., 914., 915., 916., 917.,
            918., 919., 920., 921., 922., 923., 924., 925., 926., 927., 976., 977., 978., 979.,
            980., 981., 982., 983., 984., 985., 986., 987., 988., 989., 990., 991.,
        ],
        device,
        512..768,
    );
}

/// Exported test
pub fn load_shared_memory_lhs_k_offset_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Lhs,
        DimsTestCase {
            m: 64,
            k: 64,
            n: 64,
        },
        32,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0,
            46.0, 47.0, 96.0, 97.0, 98.0, 99.0, 100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0,
            107.0, 108.0, 109.0, 110.0, 111.0, 160.0, 161.0, 162.0, 163.0, 164.0, 165.0, 166.0,
            167.0, 168.0, 169.0, 170.0, 171.0, 172.0, 173.0, 174.0, 175.0, 224.0, 225.0, 226.0,
            227.0, 228.0, 229.0, 230.0, 231.0, 232.0, 233.0, 234.0, 235.0, 236.0, 237.0, 238.0,
            239.0, 288.0, 289.0, 290.0, 291.0, 292.0, 293.0, 294.0, 295.0, 296.0, 297.0, 298.0,
            299.0, 300.0, 301.0, 302.0, 303.0, 352.0, 353.0, 354.0, 355.0, 356.0, 357.0, 358.0,
            359.0, 360.0, 361.0, 362.0, 363.0, 364.0, 365.0, 366.0, 367.0, 416.0, 417.0, 418.0,
            419.0, 420.0, 421.0, 422.0, 423.0, 424.0, 425.0, 426.0, 427.0, 428.0, 429.0, 430.0,
            431.0, 480.0, 481.0, 482.0, 483.0, 484.0, 485.0, 486.0, 487.0, 488.0, 489.0, 490.0,
            491.0, 492.0, 493.0, 494.0, 495.0, 544.0, 545.0, 546.0, 547.0, 548.0, 549.0, 550.0,
            551.0, 552.0, 553.0, 554.0, 555.0, 556.0, 557.0, 558.0, 559.0, 608.0, 609.0, 610.0,
            611.0, 612.0, 613.0, 614.0, 615.0, 616.0, 617.0, 618.0, 619.0, 620.0, 621.0, 622.0,
            623.0, 672.0, 673.0, 674.0, 675.0, 676.0, 677.0, 678.0, 679.0, 680.0, 681.0, 682.0,
            683.0, 684.0, 685.0, 686.0, 687.0, 736.0, 737.0, 738.0, 739.0, 740.0, 741.0, 742.0,
            743.0, 744.0, 745.0, 746.0, 747.0, 748.0, 749.0, 750.0, 751.0, 800.0, 801.0, 802.0,
            803.0, 804.0, 805.0, 806.0, 807.0, 808.0, 809.0, 810.0, 811.0, 812.0, 813.0, 814.0,
            815.0, 864.0, 865.0, 866.0, 867.0, 868.0, 869.0, 870.0, 871.0, 872.0, 873.0, 874.0,
            875.0, 876.0, 877.0, 878.0, 879.0, 928.0, 929.0, 930.0, 931.0, 932.0, 933.0, 934.0,
            935.0, 936.0, 937.0, 938.0, 939.0, 940.0, 941.0, 942.0, 943.0, 992.0, 993.0, 994.0,
            995.0, 996.0, 997.0, 998.0, 999.0, 1000.0, 1001.0, 1002.0, 1003.0, 1004.0, 1005.0,
            1006.0, 1007.0,
        ],
        device,
        0..256,
    );
}

/// Exported test
pub fn load_shared_memory_rhs_k_offset_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Rhs,
        DimsTestCase {
            m: 64,
            k: 64,
            n: 64,
        },
        32,
        CmmaConfig::new(B_MN, B_K, false, WriteOutStrategy::LargeSmem),
        &[
            2048., 2049., 2050., 2051., 2052., 2053., 2054., 2055., 2056., 2057., 2058., 2059.,
            2060., 2061., 2062., 2063., 2112., 2113., 2114., 2115., 2116., 2117., 2118., 2119.,
            2120., 2121., 2122., 2123., 2124., 2125., 2126., 2127., 2176., 2177., 2178., 2179.,
            2180., 2181., 2182., 2183., 2184., 2185., 2186., 2187., 2188., 2189., 2190., 2191.,
            2240., 2241., 2242., 2243., 2244., 2245., 2246., 2247., 2248., 2249., 2250., 2251.,
            2252., 2253., 2254., 2255., 2304., 2305., 2306., 2307., 2308., 2309., 2310., 2311.,
            2312., 2313., 2314., 2315., 2316., 2317., 2318., 2319., 2368., 2369., 2370., 2371.,
            2372., 2373., 2374., 2375., 2376., 2377., 2378., 2379., 2380., 2381., 2382., 2383.,
            2432., 2433., 2434., 2435., 2436., 2437., 2438., 2439., 2440., 2441., 2442., 2443.,
            2444., 2445., 2446., 2447., 2496., 2497., 2498., 2499., 2500., 2501., 2502., 2503.,
            2504., 2505., 2506., 2507., 2508., 2509., 2510., 2511., 2560., 2561., 2562., 2563.,
            2564., 2565., 2566., 2567., 2568., 2569., 2570., 2571., 2572., 2573., 2574., 2575.,
            2624., 2625., 2626., 2627., 2628., 2629., 2630., 2631., 2632., 2633., 2634., 2635.,
            2636., 2637., 2638., 2639., 2688., 2689., 2690., 2691., 2692., 2693., 2694., 2695.,
            2696., 2697., 2698., 2699., 2700., 2701., 2702., 2703., 2752., 2753., 2754., 2755.,
            2756., 2757., 2758., 2759., 2760., 2761., 2762., 2763., 2764., 2765., 2766., 2767.,
            2816., 2817., 2818., 2819., 2820., 2821., 2822., 2823., 2824., 2825., 2826., 2827.,
            2828., 2829., 2830., 2831., 2880., 2881., 2882., 2883., 2884., 2885., 2886., 2887.,
            2888., 2889., 2890., 2891., 2892., 2893., 2894., 2895., 2944., 2945., 2946., 2947.,
            2948., 2949., 2950., 2951., 2952., 2953., 2954., 2955., 2956., 2957., 2958., 2959.,
            3008., 3009., 3010., 3011., 3012., 3013., 3014., 3015., 3016., 3017., 3018., 3019.,
            3020., 3021., 3022., 3023.,
        ],
        device,
        0..256,
    );
}

/// Exported test
pub fn load_shared_memory_rhs_larger_block_test<R: Runtime>(device: &R::Device) {
    load_shared_memory_test_case::<R>(
        InputTensor::Rhs,
        DimsTestCase {
            m: 16,
            k: 32,
            n: 32,
        },
        0,
        CmmaConfig::new(32, 32, false, WriteOutStrategy::LargeSmem),
        &[
            0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0,
            46.0, 47.0, 64.0, 65.0, 66.0, 67.0, 68.0, 69.0, 70.0, 71.0, 72.0, 73.0, 74.0, 75.0,
            76.0, 77.0, 78.0, 79.0, 96.0, 97.0, 98.0, 99.0, 100.0, 101.0, 102.0, 103.0, 104.0,
            105.0, 106.0, 107.0, 108.0, 109.0, 110.0, 111.0, 128.0, 129.0, 130.0, 131.0, 132.0,
            133.0, 134.0, 135.0, 136.0, 137.0, 138.0, 139.0, 140.0, 141.0, 142.0, 143.0, 160.0,
            161.0, 162.0, 163.0, 164.0, 165.0, 166.0, 167.0, 168.0, 169.0, 170.0, 171.0, 172.0,
            173.0, 174.0, 175.0, 192.0, 193.0, 194.0, 195.0, 196.0, 197.0, 198.0, 199.0, 200.0,
            201.0, 202.0, 203.0, 204.0, 205.0, 206.0, 207.0, 224.0, 225.0, 226.0, 227.0, 228.0,
            229.0, 230.0, 231.0, 232.0, 233.0, 234.0, 235.0, 236.0, 237.0, 238.0, 239.0, 256.0,
            257.0, 258.0, 259.0, 260.0, 261.0, 262.0, 263.0, 264.0, 265.0, 266.0, 267.0, 268.0,
            269.0, 270.0, 271.0, 288.0, 289.0, 290.0, 291.0, 292.0, 293.0, 294.0, 295.0, 296.0,
            297.0, 298.0, 299.0, 300.0, 301.0, 302.0, 303.0, 320.0, 321.0, 322.0, 323.0, 324.0,
            325.0, 326.0, 327.0, 328.0, 329.0, 330.0, 331.0, 332.0, 333.0, 334.0, 335.0, 352.0,
            353.0, 354.0, 355.0, 356.0, 357.0, 358.0, 359.0, 360.0, 361.0, 362.0, 363.0, 364.0,
            365.0, 366.0, 367.0, 384.0, 385.0, 386.0, 387.0, 388.0, 389.0, 390.0, 391.0, 392.0,
            393.0, 394.0, 395.0, 396.0, 397.0, 398.0, 399.0, 416.0, 417.0, 418.0, 419.0, 420.0,
            421.0, 422.0, 423.0, 424.0, 425.0, 426.0, 427.0, 428.0, 429.0, 430.0, 431.0, 448.0,
            449.0, 450.0, 451.0, 452.0, 453.0, 454.0, 455.0, 456.0, 457.0, 458.0, 459.0, 460.0,
            461.0, 462.0, 463.0, 480.0, 481.0, 482.0, 483.0, 484.0, 485.0, 486.0, 487.0, 488.0,
            489.0, 490.0, 491.0, 492.0, 493.0, 494.0, 495.0, 512.0, 513.0, 514.0, 515.0, 516.0,
            517.0, 518.0, 519.0, 520.0, 521.0, 522.0, 523.0, 524.0, 525.0, 526.0, 527.0, 544.0,
            545.0, 546.0, 547.0, 548.0, 549.0, 550.0, 551.0, 552.0, 553.0, 554.0, 555.0, 556.0,
            557.0, 558.0, 559.0, 576.0, 577.0, 578.0, 579.0, 580.0, 581.0, 582.0, 583.0, 584.0,
            585.0, 586.0, 587.0, 588.0, 589.0, 590.0, 591.0, 608.0, 609.0, 610.0, 611.0, 612.0,
            613.0, 614.0, 615.0, 616.0, 617.0, 618.0, 619.0, 620.0, 621.0, 622.0, 623.0, 640.0,
            641.0, 642.0, 643.0, 644.0, 645.0, 646.0, 647.0, 648.0, 649.0, 650.0, 651.0, 652.0,
            653.0, 654.0, 655.0, 672.0, 673.0, 674.0, 675.0, 676.0, 677.0, 678.0, 679.0, 680.0,
            681.0, 682.0, 683.0, 684.0, 685.0, 686.0, 687.0, 704.0, 705.0, 706.0, 707.0, 708.0,
            709.0, 710.0, 711.0, 712.0, 713.0, 714.0, 715.0, 716.0, 717.0, 718.0, 719.0, 736.0,
            737.0, 738.0, 739.0, 740.0, 741.0, 742.0, 743.0, 744.0, 745.0, 746.0, 747.0, 748.0,
            749.0, 750.0, 751.0, 768.0, 769.0, 770.0, 771.0, 772.0, 773.0, 774.0, 775.0, 776.0,
            777.0, 778.0, 779.0, 780.0, 781.0, 782.0, 783.0, 800.0, 801.0, 802.0, 803.0, 804.0,
            805.0, 806.0, 807.0, 808.0, 809.0, 810.0, 811.0, 812.0, 813.0, 814.0, 815.0, 832.0,
            833.0, 834.0, 835.0, 836.0, 837.0, 838.0, 839.0, 840.0, 841.0, 842.0, 843.0, 844.0,
            845.0, 846.0, 847.0, 864.0, 865.0, 866.0, 867.0, 868.0, 869.0, 870.0, 871.0, 872.0,
            873.0, 874.0, 875.0, 876.0, 877.0, 878.0, 879.0, 896.0, 897.0, 898.0, 899.0, 900.0,
            901.0, 902.0, 903.0, 904.0, 905.0, 906.0, 907.0, 908.0, 909.0, 910.0, 911.0, 928.0,
            929.0, 930.0, 931.0, 932.0, 933.0, 934.0, 935.0, 936.0, 937.0, 938.0, 939.0, 940.0,
            941.0, 942.0, 943.0, 960.0, 961.0, 962.0, 963.0, 964.0, 965.0, 966.0, 967.0, 968.0,
            969.0, 970.0, 971.0, 972.0, 973.0, 974.0, 975.0, 992.0, 993.0, 994.0, 995.0, 996.0,
            997.0, 998.0, 999.0, 1000.0, 1001.0, 1002.0, 1003.0, 1004.0, 1005.0, 1006.0, 1007.0,
        ],
        device,
        0..512,
    );
}
