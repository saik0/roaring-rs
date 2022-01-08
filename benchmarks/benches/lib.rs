mod datasets_paths;
mod prefetched_datasets_paths;

use itertools::Itertools;
// use std::cmp::Reverse;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::{fs, io};

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Bencher, Criterion};
use prefetched_datasets_paths::*;
use roaring::RoaringBitmap;

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

// fn len(c: &mut Criterion) {
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
//         c.bench_function(&format!("{}/len", filename), |b| {
//             b.iter(|| {
//                 for a in bitmaps.iter() {
//                     a.len();
//                 }
//             });
//         });
//     }
// }

fn and(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
        c.bench_function(&format!("{}/and", filename), |b| {
            b.iter(|| {
                for (a, b) in bitmaps.iter().tuple_windows::<(_, _)>() {
                    black_box(a & b);
                }
            });
        });
    }
}

// fn intersect_with(c: &mut Criterion) {
//     c.bench_function("intersect_with", |b| {
//         let mut bitmap1: RoaringBitmap = (1..100).collect();
//         let bitmap2: RoaringBitmap = (100..200).collect();
//
//         b.iter(|| {
//             bitmap1 &= black_box(&bitmap2);
//         });
//     });
// }

fn or(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
        c.bench_function(&format!("{}/or", filename), |b| {
            b.iter(|| {
                for (a, b) in bitmaps.iter().tuple_windows::<(_, _)>() {
                    black_box(a | b);
                }
            });
        });
    }
}

// fn union_with(c: &mut Criterion) {
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
//         c.bench_function("union_with", |b| {
//             for (a, b) in bitmaps.iter().tuple_windows::<(_, _)>() {
//                 black_box(a | b);
//             }
//         });
//     }
// }

fn xor(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
        c.bench_function(&format!("{}/is_subset", filename), |b| {
            b.iter(|| {
                for (a, b) in bitmaps.iter().tuple_windows::<(_, _)>() {
                    (a ^ b);
                }
            });
        });
    }
}

// fn symmetric_deference_with(c: &mut Criterion) {
//     c.bench_function("symmetric_deference_with", |b| {
//         let mut bitmap1: RoaringBitmap = (1..100).collect();
//         let bitmap2: RoaringBitmap = (100..200).collect();
//
//         b.iter(|| {
//             bitmap1 ^= black_box(&bitmap2);
//         });
//     });
// }

fn is_subset(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
        c.bench_function(&format!("{}/is_subset", filename), |b| {
            b.iter(|| {
                for (a, b) in bitmaps.iter().tuple_windows::<(_, _)>() {
                    black_box(a.is_subset(b));
                }
            });
        });
    }
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

fn iter(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
        c.bench_function(&format!("{}/iter", filename), |b| {
            b.iter(|| {
                for bitmap in &bitmaps {
                    for value in bitmap.iter() {
                        black_box(value);
                    }
                }
            });
        });
    }
}

fn serialize(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
        c.bench_function(&format!("{}/serialize", filename), |b| {
            let capacity = bitmaps.iter().map(|b| b.serialized_size()).max().unwrap();
            let mut buffer = Vec::with_capacity(capacity);
            b.iter(|| {
                black_box(&buffer);
                for bitmap in &bitmaps {
                    bitmap.serialize_into(&mut buffer).unwrap();
                }
                buffer.clear();
                black_box(&buffer);
            });
        });
    }
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

fn extract_integers<A: AsRef<str>>(content: A) -> Result<Vec<u32>, ParseIntError> {
    content.as_ref().split(',').map(|s| s.trim().parse()).collect()
}

// Parse every file into a vector of integer.
fn parse_dir_files<A: AsRef<Path>>(
    files: A,
) -> io::Result<Vec<(PathBuf, Result<Vec<u32>, ParseIntError>)>> {
    fs::read_dir(files)?
        .map(|r| r.and_then(|e| fs::read_to_string(e.path()).map(|r| (e.path(), r))))
        .map(|r| r.map(|(p, c)| (p, extract_integers(c))))
        .collect()
}

fn from_sorted_iter(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            (files, parsed_numbers)
        })
        .collect::<Vec<_>>();

    for (filename, parsed_numbers) in datasets {
        c.bench_function(&format!("{}/from_sorted_iter", filename), |b| {
            b.iter(|| {
                for (_, numbers) in &parsed_numbers {
                    let numbers = numbers.as_ref().unwrap();
                    black_box(RoaringBitmap::from_sorted_iter(numbers.iter().copied()));
                }
            })
        });
    }
}

fn successive_and(c: &mut Criterion) {
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
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
    const data_dirs: &[&str] =
        &[CENSUS1881_SRT, CENSUS_INCOME_SRT, WEATHER_SEPT_85_SRT, WIKILEAKS_NOQUOTES_SRT];

    let datasets = data_dirs
        .iter()
        .map(|files| {
            let parsed_numbers = parse_dir_files(files).unwrap();
            let bitmaps: Vec<_> = parsed_numbers
                .into_iter()
                .map(|(_, r)| r.map(RoaringBitmap::from_sorted_iter).unwrap().unwrap())
                .collect();
            (files, bitmaps)
        })
        .collect::<Vec<_>>();

    for (filename, bitmaps) in datasets {
        let mut group = c.benchmark_group("Successive Or");
        group.bench_function(&format!("{}/Successive Or Assign Ref", filename), |b| {
            b.iter(|| {
                let mut output = RoaringBitmap::new();
                for bitmap in &bitmaps {
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
                for bitmap in &bitmaps {
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
    // len,
    and,
    // intersect_with,
    or,
    // union_with,
    xor,
    // symmetric_deference_with,
    is_subset,
    // remove,
    // remove_range_bitmap,
    // insert_range_bitmap,
    iter,
    serialize,
    // serialized_size,
    from_sorted_iter,
    successive_and,
    successive_or,
);
criterion_main!(benches);
