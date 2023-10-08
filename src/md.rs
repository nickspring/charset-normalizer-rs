use cached::proc_macro::cached;
use log::trace;
use ordered_float::OrderedFloat;

pub(crate) mod plugins;
pub(crate) mod structs;

use plugins::{
    ArchaicUpperLowerPlugin, CjkInvalidStopPlugin, MessDetectorPlugin, SuperWeirdWordPlugin,
    SuspiciousDuplicateAccentPlugin, SuspiciousRangePlugin, TooManyAccentuatedPlugin,
    TooManySymbolOrPunctuationPlugin, UnprintablePlugin,
};
use structs::MessDetectorChar;

//
// Mess detection module
//

// Compute a mess ratio given a decoded bytes sequence. The maximum threshold does stop the computation earlier.
#[cached(size = 2048)]
pub(crate) fn mess_ratio(
    decoded_sequence: String,
    maximum_threshold: Option<OrderedFloat<f32>>,
) -> f32 {
    let maximum_threshold = f32::from(maximum_threshold.unwrap_or(OrderedFloat(0.2)));
    let mut detectors: Vec<Box<dyn MessDetectorPlugin>> = vec![
        Box::<TooManySymbolOrPunctuationPlugin>::default(),
        Box::<TooManyAccentuatedPlugin>::default(),
        Box::<UnprintablePlugin>::default(),
        Box::<SuspiciousRangePlugin>::default(),
        Box::<SuspiciousDuplicateAccentPlugin>::default(),
        Box::<SuperWeirdWordPlugin>::default(),
        Box::<CjkInvalidStopPlugin>::default(),
        Box::<ArchaicUpperLowerPlugin>::default(),
    ];

    let length = decoded_sequence.chars().count();
    let mut mean_mess_ratio: Option<f32> = None;
    let early_calc_period: usize = match length {
        ..=510 => 32,
        511..=1023 => 64,
        _ => 128,
    };
    // Traverse through chars and detectors
    for (index, ch) in decoded_sequence
        .chars()
        .chain(std::iter::once('\n'))
        .enumerate()
    {
        let mess_char = MessDetectorChar::new(ch);
        detectors
            .iter_mut()
            .filter(|detector| detector.eligible(&mess_char))
            .for_each(|detector| detector.feed(&mess_char));

        if index.rem_euclid(early_calc_period) == early_calc_period - 1 {
            let early_mess_ratio: f32 = detectors.iter().map(|x| x.ratio()).sum();
            if early_mess_ratio >= maximum_threshold {
                mean_mess_ratio = Some(early_mess_ratio);
                break;
            }
        }
    }
    let return_ratio = mean_mess_ratio.unwrap_or(detectors.iter().map(|x| x.ratio()).sum());
    trace!(
        "Mess-detector extended-analysis start: \
        early_calc_period={}, \
        mean_mess_ratio={}, \
        maximum_threshold={}",
        early_calc_period,
        return_ratio,
        maximum_threshold,
    );

    /*if decoded_sequence.len() > 16 {
        trace!(
            "Chunk: {} ..... {}",
            &decoded_sequence[..decoded_sequence
                .char_indices()
                .nth(16)
                .map(|(i, _)| i)
                .unwrap_or(decoded_sequence.chars().count())],
            &decoded_sequence[decoded_sequence
                .char_indices()
                .nth(decoded_sequence.chars().count() - 16)
                .map(|(i, _)| i)
                .unwrap_or(decoded_sequence.chars().count())..],
        );
    }
     */

    for detector in &detectors {
        if detector.ratio() > 0.0 {
            trace!("{} produces ratio: {}", detector.name(), detector.ratio());
        }
    }
    trace!("===");
    return_ratio
}
