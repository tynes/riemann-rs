use coins_bip32::{prelude::*, primitives::Hint, xkeys::GenericXPriv};
use criterion::{criterion_group, criterion_main, Criterion};

fn derive_10_times(key: &GenericXPriv<Secp256k1>) {
    let path: [u32; 10] = [
        0,
        1,
        2,
        3,
        4,
        0x8000_0001,
        0x8000_0002,
        0x8000_0003,
        0x8000_0004,
        0x8000_0005,
    ];
    key.derive_private_path(&path[..]).unwrap();
}

pub fn bench_10(c: &mut Criterion) {
    let seed: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let xpriv = GenericXPriv::root_from_seed(&seed, Some(Hint::Legacy)).unwrap();

    c.bench_function("derive_10", |b| b.iter(|| derive_10_times(&xpriv)));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_10
}
criterion_main!(benches);
