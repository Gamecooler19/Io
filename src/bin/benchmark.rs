use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_compiler(c: &mut Criterion) {
    c.bench_function("compile hello world", |b| {
        b.iter(|| compile_file("examples/hello_world.io"))
    });

    c.bench_function("standard lib", |b| b.iter(|| run_stdlib_tests()));
}

criterion_group!(benches, benchmark_compiler);
criterion_main!(benches);
