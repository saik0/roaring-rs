mod datasets_paths;
mod prefetched_datasets_paths;

use itertools::Itertools;
use std::fs::{DirEntry, File};
use std::io::{BufReader, Read};
use std::num::ParseIntError;
use std::ops::BitOrAssign;
use std::path::{Path, PathBuf};
use std::{fs, io};

use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use croaring::Bitmap;
use lazy_static::lazy_static;
use prefetched_datasets_paths::*;
use roaring::RoaringBitmap;

type DirectoryName = &'static str;

// const DATASETS: &[&str] = &[
//     CENSUS1881,
// ];

// const DATASETS: &[&str] = &[
//     CENSUS1881,
//     CENSUS_INCOME,
//     WEATHER_SEPT_85,
//     WIKILEAKS_NOQUOTES
// ];

// const DATASETS: &[&str] = &[
//     WEATHER_SEPT_85,
// ];

const DATASETS: &[&str] = &[
    // CENSUS1881,
    // CENSUS1881_SRT,
    // CENSUS_INCOME,
    // CENSUS_INCOME_SRT,
    WEATHER_SEPT_85,
    // WEATHER_SEPT_85_SRT,
    // WIKILEAKS_NOQUOTES,
    // WIKILEAKS_NOQUOTES_SRT,
];

lazy_static! {
    static ref PARSED_DATASET_NUMBERS: Vec<(DirectoryName, Vec<(DirEntry, Vec<u32>)>)> =
        DATASETS
            .iter()
            .map(|&dir| (dir, parse_dir_files2(dir)))
            .collect();
    static ref PARSED_DATASET_BITMAPS: Vec<(DirectoryName, Vec<RoaringBitmap>)> = DATASETS
            .iter()
            .map(|&dir| (dir, parse_dir_bin(dir, "bin")))
            .collect();

    static ref PARSED_DATASET_ARRAYS: Vec<(DirectoryName, Vec<RoaringBitmap>)> = DATASETS
            .iter()
            .map(|&dir| (dir, parse_dir_bin(dir, "arrays")))
            .collect();
    //
    // static ref PARSED_DATASET_CBITMAPS: Vec<(DirectoryName, Vec<RoaringBitmap>)> = PARSED_DATASET_NUMBERS
    //     .iter()
    //     .map(|(dir, files)| {
    //         let bitmaps = files
    //             .iter()
    //             .map(|(_, parsed_numbers)| {
    //                 RoaringBitmap::from_sorted_iter(parsed_numbers.iter().cloned())
    //                     .expect(&format!("failed to parse roaring from {}", dir))
    //             })
    //             .collect();
    //
    //         (*dir, bitmaps)
    //     })
    //     .collect();
}

fn extract_integers(content: &str) -> Result<Vec<u32>, ParseIntError> {
    content.split(',').map(|s| s.trim().parse()).collect()
}

// Parse every file into a vector of integer.
fn parse_dir_files(files: &Path) -> io::Result<Vec<(PathBuf, Result<Vec<u32>, ParseIntError>)>> {
    fs::read_dir(files)?
        .map(|r| r.and_then(|e| fs::read_to_string(e.path()).map(|r| (e.path(), r))))
        .map(|r| r.map(|(p, c)| (p, extract_integers(&c))))
        .collect()
}

fn parse_dir_files2(files: &str) -> Vec<(DirEntry, Vec<u32>)> {
    let path_str = format!("datasets/numbers/{}", files);
    let path = Path::new(path_str.as_str());
    fs::read_dir(path)
        .expect(&format!("failed to read dir: {:?}", files))
        .map(|r| {
            let file = r.expect("an error ocurred while reading files");
            let str = fs::read_to_string(file.path()).expect(&format!("failed to read {:?}", file));
            let parsed_numbers =
                extract_integers(&str).expect(&format!("failed to parse int from {:?}", str));
            (file, parsed_numbers)
        })
        .sorted_unstable_by_key(|(file, _)| file.file_name())
        .collect()
}

fn parse_dir_bin(files: &str, subdir: &str) -> Vec<RoaringBitmap> {
    let path_str = format!("datasets/{}/{}", subdir, files);
    let path = Path::new(path_str.as_str());
    fs::read_dir(path)
        .expect(&format!("failed to read dir: {:?}", files))
        .map(|r| r.expect("an error ocurred while reading files"))
        .sorted_unstable_by_key(|file| file.file_name())
        .map(|file| {
            let f = File::open(file.path()).unwrap();
            let mut buf = BufReader::new(f);
            let bitmap = RoaringBitmap::deserialize_from(buf).unwrap();
            bitmap
        })
        .collect()
}

fn parse_c_dir_bin(files: &str, subdir: &str) -> Vec<Bitmap> {
    let path_str = format!("datasets/{}/{}", subdir, files);
    let path = Path::new(path_str.as_str());
    fs::read_dir(path)
        .expect(&format!("failed to read dir: {:?}", files))
        .map(|r| r.expect("an error ocurred while reading files"))
        .sorted_unstable_by_key(|file| file.file_name())
        .map(|file| {
            let f = File::open(file.path()).unwrap();
            let mut buf = BufReader::new(f);
            let mut bytes: Vec<u8> = Vec::new();
            buf.read_to_end(&mut bytes);
            let bitmap = croaring::Bitmap::deserialize(&bytes);
            bitmap
        })
        .collect()
}

// fn create(c: &mut Criterion) {
//     c.bench_function("create", |b| {
//         b.iter(|| {
//             RoaringBitmap::new();
//         })
//     });
// }
//
// fn insert(c: &mut Criterion) {
//     c.bench_function("create & insert 1", |b| {
//         b.iter(|| {
//             let mut bitmap = RoaringBitmap::new();
//             bitmap.insert(black_box(1));
//         });
//     });
//
//     c.bench_function("insert 1", |b| {
//         let mut bitmap = RoaringBitmap::new();
//         b.iter(|| {
//             bitmap.insert(black_box(1));
//         });
//     });
//
//     c.bench_function("create & insert several", |b| {
//         b.iter(|| {
//             let mut bitmap = RoaringBitmap::new();
//             bitmap.insert(black_box(1));
//             bitmap.insert(black_box(10));
//             bitmap.insert(black_box(100));
//             bitmap.insert(black_box(1_000));
//             bitmap.insert(black_box(10_000));
//             bitmap.insert(black_box(100_000));
//             bitmap.insert(black_box(1_000_000));
//         });
//     });
//
//     c.bench_function("insert several", |b| {
//         let mut bitmap = RoaringBitmap::new();
//         b.iter(|| {
//             bitmap.insert(black_box(1));
//             bitmap.insert(black_box(10));
//             bitmap.insert(black_box(100));
//             bitmap.insert(black_box(1_000));
//             bitmap.insert(black_box(10_000));
//             bitmap.insert(black_box(100_000));
//             bitmap.insert(black_box(1_000_000));
//         });
//     });
// }
//
// fn contains(c: &mut Criterion) {
//     c.bench_function("contains true", |b| {
//         let mut bitmap: RoaringBitmap = RoaringBitmap::new();
//         bitmap.insert(1);
//
//         b.iter(|| {
//             bitmap.contains(black_box(1));
//         });
//     });
//
//     c.bench_function("contains false", |b| {
//         let bitmap: RoaringBitmap = RoaringBitmap::new();
//
//         b.iter(|| {
//             bitmap.contains(black_box(1));
//         });
//     });
// }

fn binary_op(
    c: &mut Criterion,
    op_name: &str,
    op_owned: fn(RoaringBitmap, RoaringBitmap) -> RoaringBitmap,
    op_ref: fn(RoaringBitmap, &RoaringBitmap) -> RoaringBitmap,
    op_assign_owned: fn(RoaringBitmap, RoaringBitmap),
    op_assign_ref: fn(RoaringBitmap, &RoaringBitmap),
) {
    let mut group = c.benchmark_group(format!("pairwise_{}", op_name));

    for (filename, bitmaps) in PARSED_DATASET_BITMAPS.iter() {
        // Number of bits

        group.bench_function(BenchmarkId::new("own", filename), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(op_owned(a, b));
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("ref", filename), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(op_ref(a, &b));
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("assign_own", filename), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        op_assign_owned(a, b)
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("assign_ref", filename), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        op_assign_ref(a, &b)
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn and2(c: &mut Criterion) {
    let c_arrays: Vec<(DirectoryName, Vec<croaring::Bitmap>)> =
        DATASETS.iter().map(|&dir| (dir, parse_c_dir_bin(dir, "arrays"))).collect();

    let mut group = c.benchmark_group(format!("pairwise_and"));

    for (filename, bitmaps) in PARSED_DATASET_ARRAYS.iter() {
        // Number of bits

        group.bench_function(BenchmarkId::new(*filename, "cur".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a & &b);
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new(*filename, "opt_unsafe".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a.and_opt_unsafe(&b));
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new(*filename, "x86_simd".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a.and_x86_simd(&b));
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new(*filename, "std_simd".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a.and_std_simd(&b));
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    for (filename, bitmaps) in c_arrays.iter() {
        // Number of bits

        group.bench_function(BenchmarkId::new(*filename, "c".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a.and(&b));
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();

    let mut group = c.benchmark_group(format!("pairwise_and_assign"));

    for (filename, bitmaps) in PARSED_DATASET_ARRAYS.iter() {
        // Number of bits

        // group.bench_function(BenchmarkId::new(*filename, "linear".to_string()), |b| {
        //     b.iter_batched(
        //         || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
        //         |bitmaps| {
        //             for (mut a, b) in bitmaps {
        //                 a.and_assign_linear(&b);
        //             }
        //         },
        //         BatchSize::SmallInput,
        //     );
        // });

        group.bench_function(BenchmarkId::new(*filename, "cur".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (mut a, b) in bitmaps {
                        a &= &b;
                    }
                },
                BatchSize::SmallInput,
            );
        });

        // group.bench_function(BenchmarkId::new(*filename, "walk".to_string()), |b| {
        //     b.iter_batched(
        //         || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
        //         |bitmaps| {
        //             for (mut a, b) in bitmaps {
        //                 a.and_assign_walk(&b);
        //             }
        //         },
        //         BatchSize::SmallInput,
        //     );
        // });

        // group.bench_function(BenchmarkId::new(*filename, "run".to_string()), |b| {
        //     b.iter_batched(
        //         || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
        //         |bitmaps| {
        //             for (mut a, b) in bitmaps {
        //                 a.and_assign_run(&b);
        //             }
        //         },
        //         BatchSize::SmallInput,
        //     );
        // });
        //
        // group.bench_function(BenchmarkId::new(*filename, "gallop".to_string()), |b| {
        //     b.iter_batched(
        //         || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
        //         |bitmaps| {
        //             for (mut a, b) in bitmaps {
        //                 a.and_assign_gallop(&b);
        //             }
        //         },
        //         BatchSize::SmallInput,
        //     );
        // });

        // group.bench_function(BenchmarkId::new(*filename, "opt".to_string()), |b| {
        //     b.iter_batched(
        //         || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
        //         |bitmaps| {
        //             for (mut a, b) in bitmaps {
        //                 a.and_assign_opt(&b);
        //             }
        //         },
        //         BatchSize::SmallInput,
        //     );
        // });

        group.bench_function(BenchmarkId::new(*filename, "opt_unsafe".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (mut a, b) in bitmaps {
                        a.and_assign_opt_unsafe(&b);
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new(*filename, "x86_simd".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (mut a, b) in bitmaps {
                        a.and_assign_x86_simd(&b);
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new(*filename, "std_simd".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (mut a, b) in bitmaps {
                        a.and_assign_std_simd(&b);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    for (filename, bitmaps) in c_arrays.iter() {
        // Number of bits

        group.bench_function(BenchmarkId::new(*filename, "c".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (mut a, b) in bitmaps {
                        a.and_inplace(&b);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn or2(c: &mut Criterion) {
    let c_arrays: Vec<(DirectoryName, Vec<croaring::Bitmap>)> =
        DATASETS.iter().map(|&dir| (dir, parse_c_dir_bin(dir, "arrays"))).collect();

    let mut group = c.benchmark_group(format!("pairwise_or"));

    for (filename, bitmaps) in PARSED_DATASET_ARRAYS.iter() {
        // Number of bits

        group.bench_function(BenchmarkId::new(*filename, "rs".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a | &b);
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new(*filename, "x86".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a.or_x86_simd(&b));
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    for (filename, bitmaps) in c_arrays.iter() {
        // Number of bits

        group.bench_function(BenchmarkId::new(*filename, "c".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        black_box(a.or(&b));
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();

    let mut group = c.benchmark_group(format!("pairwise_or_assign"));

    for (filename, bitmaps) in PARSED_DATASET_ARRAYS.iter() {
        // Number of bits

        group.bench_function(BenchmarkId::new(*filename, "rs".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (ref mut a, ref b) in bitmaps {
                        a.union_with(b);
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new(*filename, "x86".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (ref mut a, ref b) in bitmaps {
                        a.or_assign_x86_simd(b)
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    for (filename, bitmaps) in c_arrays.iter() {
        // Number of bits
        group.bench_function(BenchmarkId::new(*filename, "c".to_string()), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (ref mut a, ref b) in bitmaps {
                        a.or_inplace(b);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// Creation

fn creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    for (filename, parsed_numbers) in PARSED_DATASET_NUMBERS.iter() {
        let count = parsed_numbers.iter().map(|(_, n)| n.len() as u64).sum();
        group.throughput(Throughput::Elements(count));

        group.bench_function(BenchmarkId::new("from_sorted_iter", filename), |b| {
            b.iter_batched(
                || parsed_numbers.iter().map(|(_, n)| n.clone()).collect::<Vec<Vec<u32>>>(),
                |parsed_numbers| {
                    for numbers in parsed_numbers {
                        black_box(RoaringBitmap::from_sorted_iter(numbers.into_iter()));
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("collect", filename), |b| {
            b.iter_batched(
                || parsed_numbers.iter().map(|(_, n)| n.clone()).collect::<Vec<Vec<u32>>>(),
                |parsed_numbers| {
                    for numbers in parsed_numbers {
                        black_box(numbers.iter().copied().collect::<RoaringBitmap>());
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// Cardinality

fn cardinality(c: &mut Criterion) {
    let mut group = c.benchmark_group("cardinality");

    for (filename, bitmaps) in PARSED_DATASET_BITMAPS.iter() {
        group.bench_function(BenchmarkId::new("len", filename), |b| {
            b.iter(|| {
                for a in bitmaps.iter() {
                    a.len();
                }
            });
        });
    }

    group.finish();
}

// Ops

fn and(c: &mut Criterion) {
    binary_op(c, "and", |a, b| a & b, |a, b| a & b, |mut a, b| a &= b, |mut a, b| a &= b)
}

fn or(c: &mut Criterion) {
    binary_op(c, "or", |a, b| a | b, |a, b| a | b, |mut a, b| a |= b, |mut a, b| a |= b)
}

fn xor(c: &mut Criterion) {
    binary_op(c, "xor", |a, b| a ^ b, |a, b| a ^ b, |mut a, b| a ^= b, |mut a, b| a ^= b)
}

fn sub(c: &mut Criterion) {
    binary_op(c, "sub", |a, b| a - b, |a, b| a - b, |mut a, b| a -= b, |mut a, b| a -= b)
}

// cmp

fn comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison");

    for (filename, bitmaps) in PARSED_DATASET_BITMAPS.iter() {
        group.bench_function(BenchmarkId::new("is_disjoint", filename), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        a.is_disjoint(&b);
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("is_subset", filename), |b| {
            b.iter_batched(
                || bitmaps.iter().cloned().tuple_windows::<(_, _)>().collect::<Vec<_>>(),
                |bitmaps| {
                    for (a, b) in bitmaps {
                        a.is_subset(&b);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// fn remove(c: &mut Criterion) {
//     c.bench_function("remove 1", |b| {
//         let mut sub: RoaringBitmap = (0..65_536).collect();
//         b.iter(|| {
//             black_box(sub.remove(1000));
//         });
//     });
// }

// fn remove_range_bitmap(c: &mut Criterion) {
//     c.bench_function("remove_range 1", |b| {
//         let mut sub: RoaringBitmap = (0..65_536).collect();
//         b.iter(|| {
//             // carefully delete part of the bitmap
//             // only the first iteration will actually change something
//             // but the runtime remains identical afterwards
//             black_box(sub.remove_range(4096 + 1..65_536));
//             assert_eq!(sub.len(), 4096 + 1);
//         });
//     });
//
//     c.bench_function("remove_range 2", |b| {
//         // Slower bench that creates a new bitmap on each iteration so that can benchmark
//         // bitmap to array conversion
//         b.iter(|| {
//             let mut sub: RoaringBitmap = (0..65_536).collect();
//             black_box(sub.remove_range(100..65_536));
//             assert_eq!(sub.len(), 100);
//         });
//     });
// }
//
// fn insert_range_bitmap(c: &mut Criterion) {
//     for &size in &[10u32, 100, 1_000, 5_000, 10_000, 20_000] {
//         let mut group = c.benchmark_group("insert_range");
//         group.throughput(criterion::Throughput::Elements(size as u64));
//         group.bench_function(&format!("from_empty_{}", size), |b| {
//             let bm = RoaringBitmap::new();
//             b.iter_batched(
//                 || bm.clone(),
//                 |mut bm| black_box(bm.insert_range(0..size)),
//                 criterion::BatchSize::SmallInput,
//             )
//         });
//         group.bench_function(format!("pre_populated_{}", size), |b| {
//             let mut bm = RoaringBitmap::new();
//             bm.insert_range(0..size);
//             b.iter_batched(
//                 || bm.clone(),
//                 |mut bm| black_box(bm.insert_range(0..size)),
//                 criterion::BatchSize::SmallInput,
//             )
//         });
//     }
// }

// Iter

fn iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("iteration");
    for (filename, bitmaps) in PARSED_DATASET_BITMAPS.iter() {
        group.bench_function(BenchmarkId::new("iter", filename), |b| {
            b.iter_batched(
                || bitmaps.clone(),
                |bitmaps| {
                    for bitmap in bitmaps {
                        for value in bitmap.iter() {
                            black_box(value);
                        }
                    }
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_function(BenchmarkId::new("into_iter", filename), |b| {
            b.iter_batched(
                || bitmaps.clone(),
                |bitmaps| {
                    for bitmap in bitmaps {
                        for value in bitmap.into_iter() {
                            black_box(value);
                        }
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// Serde

fn serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde");
    for (filename, bitmaps) in PARSED_DATASET_BITMAPS.iter() {
        group.bench_function(BenchmarkId::new("serialize", filename), |b| {
            let capacity = bitmaps.iter().map(|b| b.serialized_size()).max().unwrap();
            let mut buffer = Vec::with_capacity(capacity);
            b.iter(|| {
                buffer.clear();
                for bitmap in bitmaps {
                    bitmap.serialize_into(&mut buffer).unwrap();
                }
            });
        });

        group.bench_function(BenchmarkId::new("deserialize", filename), |b| {
            b.iter_batched(
                || {
                    bitmaps
                        .iter()
                        .map(|b| {
                            let mut buffer = Vec::with_capacity(b.serialized_size());
                            b.serialize_into(&mut buffer).unwrap();
                            buffer
                        })
                        .collect::<Vec<Vec<u8>>>()
                },
                |serialied_bitmaps| {
                    for bytes in serialied_bitmaps {
                        black_box(RoaringBitmap::deserialize_from(bytes.as_slice()).unwrap());
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

// fn serialized_size(c: &mut Criterion) {
//     const data_dirs: &[&str] =
//         &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];
//
//     let datasets = data_dirs
//         .iter()
//         .map(|files| {
//             let parsed_numbers = parse_dir_files(files).unwrap();
//             let bitmaps: Vec<_> = parsed_numbers
//                 .into_iter()
//                 .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
//                 .collect();
//             (files, bitmaps)
//         })
//         .collect::<Vec<_>>();
//
//     for (filename, bitmaps) in datasets {
//         c.bench_function(&format!("{}/serialized_size", filename), |b| {
//             b.iter(|| {
//                for bitmap in &bitmaps {
//                    bitmap.serialized_size();
//                }
//             });
//         });
//     }
// }

fn successive_and(c: &mut Criterion) {
    for (filename, bitmaps) in PARSED_DATASET_BITMAPS.iter() {
        let mut group = c.benchmark_group("Successive And");

        group.bench_function(&format!("{}/Successive And Assign Ref", filename), |b| {
            b.iter_batched(
                || bitmaps.clone(),
                |bitmaps| {
                    let mut iter = bitmaps.into_iter();
                    let mut first = iter.next().unwrap().clone();
                    for bitmap in iter {
                        first &= bitmap;
                    }
                    black_box(first);
                },
                BatchSize::LargeInput,
            );
        });

        group.bench_function(&format!("{}/Successive And Assign Owned", filename), |b| {
            b.iter_batched(
                || bitmaps.clone(),
                |bitmaps| {
                    black_box(bitmaps.into_iter().reduce(|a, b| a & b).unwrap());
                },
                BatchSize::LargeInput,
            );
        });

        group.bench_function(&format!("{}/Successive And Ref Ref", filename), |b| {
            b.iter_batched(
                || bitmaps.clone(),
                |bitmaps| {
                    let mut iter = bitmaps.iter();
                    let first = iter.next().unwrap().clone();
                    black_box(iter.fold(first, |acc, x| (&acc) & x));
                },
                BatchSize::LargeInput,
            );
        });

        group.finish();
    }
}

fn successive_or(c: &mut Criterion) {
    for (filename, bitmaps) in PARSED_DATASET_BITMAPS.iter() {
        let mut group = c.benchmark_group("Successive Or");
        group.bench_function(&format!("{}/Successive Or Assign Ref", filename), |b| {
            b.iter(|| {
                let mut output = RoaringBitmap::new();
                for bitmap in bitmaps {
                    output |= bitmap;
                }
            });
        });

        group.bench_function(&format!("{}/Successive Or Assign Owned", filename), |b| {
            b.iter_batched(
                || bitmaps.clone(),
                |bitmaps: Vec<RoaringBitmap>| {
                    let mut output = RoaringBitmap::new();
                    for bitmap in bitmaps {
                        output |= bitmap;
                    }
                },
                BatchSize::LargeInput,
            );
        });

        group.bench_function(&format!("{}/Successive Or Ref Ref", filename), |b| {
            b.iter(|| {
                let mut output = RoaringBitmap::new();
                for bitmap in bitmaps {
                    output = (&output) | bitmap;
                }
            });
        });

        group.finish();
    }
}

criterion_group!(
    benches,
    // create,
    // insert,
    // contains,
    creation,
    cardinality,
    and,
    or,
    xor,
    sub,
    comparison,
    // remove,
    // remove_range_bitmap,
    // insert_range_bitmap,
    iteration,
    serde,
    // serialized_size,
    // successive_and,
    // successive_or,
);

criterion_group!(ops, and, or, xor, sub);
// criterion_group!(ops, or2);

criterion_group!(create, creation);

criterion_main!(ops);
