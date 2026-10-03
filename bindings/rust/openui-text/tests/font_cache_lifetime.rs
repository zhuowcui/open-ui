use openui_text::font::{FontCache, FontDescription};

// This integration test runs in its own process. Other text tests may use the
// same global Skia cache in parallel, so its lifetime counts cannot be checked
// from the crate's shared unit-test process.
#[test]
fn fonts_outlive_their_collection_and_last_client_retires_cached_strikes() {
    assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);
    let description = FontDescription::default();
    let mut first = FontCache::new();
    let first_font = first
        .get_font_platform_data("sans-serif", &description)
        .expect("installed sans-serif font");
    let mut second = FontCache::new();
    let second_font = second
        .get_font_platform_data("sans-serif", &description)
        .expect("installed sans-serif font");
    let metrics = second_font.sk_font().metrics().0;
    let used = skia_safe::graphics::font_cache_count_used();
    assert!(used > 0);

    drop(first);
    drop(first_font);
    assert_eq!(skia_safe::graphics::font_cache_count_used(), used);
    assert_eq!(second_font.sk_font().metrics().0, metrics);

    drop(second);
    // A caller owns this resolved font beyond the selecting collection's life.
    assert_eq!(skia_safe::graphics::font_cache_count_used(), used);
    assert_eq!(second_font.sk_font().metrics().0, metrics);
    drop(second_font);
    assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);

    // Repeated construction/retirement must not accumulate cached typefaces.
    for _ in 0..3 {
        let mut cache = FontCache::new();
        let font = cache
            .get_font_platform_data("sans-serif", &description)
            .expect("installed sans-serif font");
        assert_eq!(font.sk_font().metrics().0, metrics);
        assert!(skia_safe::graphics::font_cache_count_used() > 0);
        drop(font);
        drop(cache);
        assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);
    }

    // Cache retirement races with collection acquisition on other threads.
    // Each thread keeps an owned font beyond the collection's lifetime.
    let start = std::sync::Arc::new(std::sync::Barrier::new(8));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let start = start.clone();
            let description = description.clone();
            scope.spawn(move || {
                start.wait();
                for _ in 0..16 {
                    let mut cache = FontCache::new();
                    let font = cache
                        .get_font_platform_data("sans-serif", &description)
                        .expect("installed sans-serif font");
                    drop(cache);
                    assert_eq!(font.sk_font().metrics().0, metrics);
                    std::thread::yield_now();
                    drop(font);
                }
            });
        }
    });
    assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);
}
