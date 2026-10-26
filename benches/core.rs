use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::{hint::black_box, time::Duration};
use vrx::{
    engine::{Playback, Timing},
    text::{Document, orp::Strategy},
};
fn benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenization");
    group
        .sample_size(20)
        .measurement_time(Duration::from_secs(3));
    for words in [10_000, 100_000, 1_000_000] {
        let source = "Reading e\u{301}moji 👨‍👩‍👧‍👦.\n\n".repeat(words / 3);
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(words), &source, |b, s| {
            b.iter_batched(
                || s.clone(),
                |owned| black_box(Document::parse(owned, &Strategy::Classic)),
                criterion::BatchSize::LargeInput,
            )
        });
    }
    group.finish();
    let document = match Document::parse("word ".repeat(100_000), &Strategy::Classic) {
        Ok(d) => d.shared(),
        Err(_) => return,
    };
    let mut group = c.benchmark_group("playback");
    group
        .sample_size(20)
        .measurement_time(Duration::from_secs(2));
    for wpm in [200, 400, 800, 1200, 2000, 4000] {
        let mut playback = Playback::new(
            document.clone(),
            Timing {
                wpm,
                adaptive: false,
                ..Timing::default()
            },
        );
        playback.play(Duration::ZERO);
        group.bench_function(BenchmarkId::from_parameter(wpm), |b| {
            b.iter(|| {
                let playback = black_box(&mut playback);
                if let Some(ticket) = playback.ticket() {
                    black_box(playback.advance(ticket, ticket.deadline));
                } else {
                    playback.restart(Duration::ZERO);
                    playback.play(Duration::ZERO);
                }
            })
        });
    }
    group.finish();
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
