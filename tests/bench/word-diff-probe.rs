// Isolates word-diff cost from renderer changes; see scripts/measure-word-diff.ps1.
#[cfg(word_diff)]
#[path = "../../src/diff.rs"]
mod diff;
#[path = "../../src/memory/mod.rs"]
mod memory;

fn main() -> std::io::Result<()> {
    let lines: Vec<(String, String)> = (0..100)
        .map(|index| {
            (
                format!("let value_{index} = old_name + unchanged; // Tiếng Việt 👨‍👩‍👧‍👦\t\r\n"),
                format!("let value_{index} = new_name + unchanged; // Tiếng Việt 👍🏽\t\r\n"),
            )
        })
        .collect();
    let mut peak = memory::current_bytes()?;
    // 30 files, loading and processing one 100-line file at a time.
    for _ in 0..30 {
        for (old, new) in &lines {
            #[cfg(word_diff)]
            let spans = diff::word_spans(std::hint::black_box(old), std::hint::black_box(new))
                .expect("bounded word-diff workload");
            #[cfg(word_diff)]
            std::hint::black_box((&spans, diff::MAX_BLOCK_LINES));
            #[cfg(not(word_diff))]
            std::hint::black_box((old, new));
            peak = peak.max(memory::current_bytes()?);
        }
    }
    let mut idle = u64::MAX;
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let sample = memory::current_bytes()?;
        idle = idle.min(sample);
        peak = peak.max(sample);
    }
    println!(
        "metric={} idle_bytes={idle} sampled_peak_bytes={peak} child_processes=0",
        memory::METRIC
    );
    Ok(())
}
