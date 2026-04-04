//! RunSegmenter — splits text into uniform runs by Unicode script.
//!
//! Mirrors Blink's `RunSegmenter` (`platform/fonts/shaping/run_segmenter.h`).
//! Groups consecutive characters with the same Unicode script into segments,
//! so each segment can be shaped with the appropriate HarfBuzz script tag.

pub use unicode_script::Script;
use unicode_script::UnicodeScript;

use super::TextDirection;

/// A contiguous run of characters sharing the same Unicode script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunSegment {
    /// Byte offset of the start in the text.
    pub start: usize,
    /// Byte offset of the end in the text (exclusive).
    pub end: usize,
    /// Unicode script of characters in this segment.
    pub script: Script,
    /// Direction of this segment.
    pub direction: TextDirection,
}

/// Segments text into runs by Unicode script.
///
/// Blink: `RunSegmenter` in `platform/fonts/shaping/run_segmenter.h`.
pub struct RunSegmenter;

impl RunSegmenter {
    /// Segment text into uniform runs by Unicode script.
    ///
    /// Characters with `Common` or `Inherited` script are merged into the
    /// adjacent run (matching Blink's RunSegmenter behavior). This prevents
    /// spaces, punctuation, and combining marks from fragmenting runs.
    ///
    /// The `direction` field is set based on the script's natural writing
    /// direction (e.g., Arabic/Hebrew → RTL). For bidi-accurate direction,
    /// use `BidiParagraph` which applies the full UAX#9 algorithm.
    pub fn segment(text: &str) -> Vec<RunSegment> {
        if text.is_empty() {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let mut current_script = Script::Common;
        let mut seg_start: usize = 0;

        for (byte_idx, ch) in text.char_indices() {
            let char_script = ch.script();

            // Resolve Common/Inherited scripts: merge them with the current
            // run's script rather than creating new segments. This matches
            // Blink's RunSegmenter behavior where spaces, punctuation, and
            // combining marks inherit the script of their context.
            let resolved = Self::resolve_script(char_script, current_script);

            if resolved != current_script && current_script != Script::Common {
                // Script changed — emit the previous segment.
                segments.push(RunSegment {
                    start: seg_start,
                    end: byte_idx,
                    script: current_script,
                    direction: Self::script_direction(current_script),
                });
                seg_start = byte_idx;
            }
            current_script = resolved;
        }

        // Emit the final segment.
        segments.push(RunSegment {
            start: seg_start,
            end: text.len(),
            script: current_script,
            direction: Self::script_direction(current_script),
        });

        segments
    }

    /// Resolve Common/Inherited scripts to the current run script.
    ///
    /// Blink: `ResolveCurrentScript` in `run_segmenter.cc`.
    fn resolve_script(char_script: Script, current_script: Script) -> Script {
        match char_script {
            Script::Common | Script::Inherited => {
                // Merge with current run — don't start a new segment.
                if current_script == Script::Common {
                    Script::Common
                } else {
                    current_script
                }
            }
            _ => char_script,
        }
    }

    /// Determine the natural writing direction for a Unicode script.
    ///
    /// RTL scripts are those whose characters are written right-to-left
    /// (Arabic, Hebrew, Syriac, Thaana, etc.). All others default to LTR.
    /// This matches Blink's `IsRtlScript` in `run_segmenter.cc`.
    fn script_direction(script: Script) -> TextDirection {
        match script {
            Script::Arabic
            | Script::Hebrew
            | Script::Syriac
            | Script::Thaana
            | Script::Mandaic
            | Script::Nko
            | Script::Samaritan
            | Script::Avestan
            | Script::Imperial_Aramaic
            | Script::Inscriptional_Pahlavi
            | Script::Inscriptional_Parthian
            | Script::Old_South_Arabian
            | Script::Old_Turkic
            | Script::Phoenician
            | Script::Adlam
            | Script::Hanifi_Rohingya
            | Script::Old_Sogdian
            | Script::Sogdian
            | Script::Yezidi
            | Script::Chorasmian
            | Script::Elymaic
            | Script::Hatran
            | Script::Old_Hungarian
            | Script::Old_Uyghur
            | Script::Mende_Kikakui
            | Script::Psalter_Pahlavi => TextDirection::Rtl,
            _ => TextDirection::Ltr,
        }
    }
}
