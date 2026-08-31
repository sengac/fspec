//! BUG-166 — pure percentage-scale math for the mux split scale.
//!
//! Feature: spec/features/mux-dividers-percentage-scale.feature
//!
//! The percentage-scale model (replaces "missing entries mean equal share"):
//! - `splits` holds ONE percent per inter-pane gap: n panes → n-1 entries;
//!   `splits[i]` is pane i's share of the available axis (after divider
//!   subtraction) in percent. The LAST pane's share is implicit:
//!   `100 - sum(splits)`, so the scale always sums to 100.
//! - [`equal_scale`] builds the default equal split for n panes
//!   (e.g. `[50]`, `[33, 33]`, `[25, 25, 25]`).
//! - [`scale_scales`] rescales an existing scale to a new pane count
//!   (adding gives new panes an equal `100/n` share; removing re-allocates
//!   the dropped share to the surviving panes proportionally).
//! - [`set_drag_pcts`] applies a dragged divider: entry `i` becomes the
//!   released percent and the panes TO ITS RIGHT absorb the change
//!   proportionally (the left panes keep their shares).
//! - Rounding is largest-remainder with ties breaking toward the LATER
//!   index, so the last pane keeps the integer remainder — the convention
//!   the layout math uses for pane widths.

/// Largest-remainder integer division: split `nums[j] / den` into integer
/// shares that sum to exactly `sum(nums) / den` (a whole number by
/// construction in every call site). Ties break toward the LATER index
/// (the integer remainder stays with the last pane).
pub fn largest_remainder(nums: &[u32], den: u32) -> Vec<u32> {
    let den = den.max(1);
    let total: u32 = nums.iter().sum::<u32>() / den;
    let mut floors: Vec<u32> = nums.iter().map(|n| n / den).collect();
    let rems: Vec<u32> = nums.iter().map(|n| n % den).collect();
    let extras = total.saturating_sub(floors.iter().sum::<u32>());
    let mut order: Vec<usize> = (0..nums.len()).collect();
    order.sort_by(|&a, &b| rems[b].cmp(&rems[a]).then(b.cmp(&a)));
    for &ix in order.iter().take(extras as usize) {
        floors[ix] += 1;
    }
    floors
}

/// Build the equal-division scale for `count` panes: `count-1` entries so
/// that the entries + the implicit last share sum to 100
/// (e.g. `[50]`, `[33, 33]`, `[25, 25, 25]`).
pub fn equal_scale(count: usize) -> Vec<u16> {
    if count <= 1 {
        return Vec::new();
    }
    let shares = largest_remainder(&vec![100u32; count], count as u32);
    shares[..count - 1]
        .iter()
        .copied()
        .map(|s| s.max(1) as u16)
        .collect()
}

/// Rescale `old` (a scale for `old.len() + 1` panes) to a scale for
/// `new_count` panes. The dropped / added panes are always the trailing
/// ones (the `/mux` grammar only keeps a prefix of the pane list).
///
/// Guarantees:
/// - returns exactly `new_count - 1` entries, each ≥ 1;
/// - entries + the implicit last share sum to 100;
/// - adding panes: every old share is scaled by `old_n / new_n`, each new
///   pane takes an equal `100 / new_n` share (equal scales stay equal);
/// - removing panes: the surviving prefix is renormalized to 100%
///   (the dropped pane's share is re-allocated proportionally).
pub fn scale_scales(old: &[u16], new_count: usize) -> Vec<u16> {
    if new_count <= 1 {
        return Vec::new();
    }
    let old_n = old.len() + 1;
    if old.is_empty() {
        return equal_scale(new_count);
    }
    // The old pane shares (old_n values summing to 100): entries plus the
    // implicit last share.
    let old_sum: u32 = old.iter().map(|p| *p as u32).sum();
    let old_shares: Vec<u32> = old
        .iter()
        .map(|p| (*p).max(1) as u32)
        .chain(std::iter::once((100 - old_sum.min(99)).max(1)))
        .collect();

    if new_count > old_n {
        // ADD: every old share × old_n/new_n; each new pane 100/new_n.
        let den = new_count as u32;
        let nums: Vec<u32> = old_shares
            .iter()
            .map(|s| *s * old_n as u32)
            .chain(std::iter::repeat_n(100u32, new_count - old_n))
            .collect();
        let shares = largest_remainder(&nums, den);
        shares[..new_count - 1]
            .iter()
            .copied()
            .map(|s| s.max(1) as u16)
            .collect()
    } else {
        // REMOVE: renormalize the surviving prefix to 100%.
        let keep = &old_shares[..new_count];
        let w: u32 = keep.iter().copied().sum::<u32>();
        if w == 0 {
            return equal_scale(new_count);
        }
        let nums: Vec<u32> = keep.iter().map(|s| *s * 100).collect();
        let shares = largest_remainder(&nums, w);
        shares[..new_count - 1]
            .iter()
            .copied()
            .map(|s| s.max(1) as u16)
            .collect()
    }
}

/// Apply a released divider: entry `i` becomes `pct`; the panes to its
/// RIGHT absorb the change proportionally (their shares + the implicit
/// last share are renormalized to `100 - left - pct`); the panes to its
/// LEFT keep their shares. Returns the new full scale.
pub fn set_drag_pcts(scale: &[u16], i: usize, pct: u16) -> Vec<u16> {
    let n = scale.len() + 1;
    if i >= n - 1 {
        return scale.to_vec();
    }
    let pct = pct.clamp(1, 99);
    let left: u32 = scale.iter().take(i).map(|p| *p as u32).sum();
    let sum: u32 = scale.iter().map(|p| *p as u32).sum();
    let last = (100 - sum).max(1);
    let target = 100u32.saturating_sub(left).saturating_sub(pct as u32);
    let slots = n - 1 - i; // entries i+1..n-2 (slots-1) + the last share
    let old_slots: Vec<u32> = scale[i + 1..]
        .iter()
        .map(|p| (*p).max(1) as u32)
        .chain(std::iter::once(last))
        .collect();
    let shares = if target < slots as u32 {
        // Defensive (should not happen with the 10..=90 drag clamp):
        // floor everything to 1, the last share may be 0 — fixed below.
        vec![1u32; slots]
    } else {
        let wsum: u32 = old_slots.iter().copied().sum::<u32>().max(slots as u32);
        largest_remainder(
            &old_slots.iter().map(|w| w * target).collect::<Vec<_>>(),
            wsum,
        )
    };
    let mut right = shares;
    if right.last() == Some(&0) {
        let last = right.len().saturating_sub(1);
        for j in (0..last).rev() {
            if right[j] > 1 {
                right[j] -= 1;
                right[last] = 1;
                break;
            }
        }
    }
    let mut out = scale[..i].to_vec();
    out.push(pct);
    for s in right.iter().take(slots - 1) {
        out.push(*s.max(&1) as u16);
    }
    out
}

/// Normalize a persisted scale for `count` panes (BUG-166):
/// - too few entries (legacy configs, e.g. `[40]` on a 3-pane grid) →
///   rescale to a full `count-1` scale ([`scale_scales`]);
/// - sum ≥ 100 (e.g. a hand-edited `[90, 90]`) → renormalize so the
///   entries + last share sum to 100;
/// - too many entries → truncate to `count-1` (then renormalize if needed).
///
/// Every entry ends up in `1..=99`.
pub fn normalize_scale(old: &[u16], count: usize) -> Vec<u16> {
    let want = count.saturating_sub(1);
    if want == 0 {
        return Vec::new();
    }
    let base: Vec<u16> = if old.len() < want {
        scale_scales(old, count)
    } else {
        old.iter().take(want).copied().collect()
    };
    let sum: u32 = base.iter().map(|p| *p as u32).sum();
    if sum == 0 {
        return equal_scale(count);
    }
    if sum <= 99 {
        return base.into_iter().map(|p| p.clamp(1, 99)).collect();
    }
    // sum > 99: renormalize the count shares to 100, entries = the first
    // count-1.
    let weights: Vec<u32> = base
        .iter()
        .map(|p| (*p).max(1) as u32)
        .chain(std::iter::once(1))
        .collect();
    let wsum: u32 = weights.iter().copied().sum::<u32>();
    let shares = largest_remainder(&weights.iter().map(|w| w * 100).collect::<Vec<_>>(), wsum);
    let mut entries: Vec<u16> = shares[..shares.len() - 1]
        .iter()
        .copied()
        .map(|s| s.max(1) as u16)
        .collect();
    // Guarantee the implicit last share stays ≥ 1: if it is 0, steal a
    // point from the largest entry.
    if *shares.last().unwrap_or(&0) == 0 {
        let idx = entries
            .iter()
            .enumerate()
            .max_by_key(|&(_, v)| *v)
            .map(|(i, _)| i)
            .unwrap_or(0);
        entries[idx] = entries[idx].max(2) - 1;
    }
    entries
}

/// True iff the scale represents an equal split: all `n` shares (the
/// `n-1` entries + the implicit last share `100 - sum`) differ by at
/// most 1 percentage point. Used by the layout math to render the
/// default grid as the EXACT equal division (`available/n`) instead of
/// flooring each 33%-style entry (which would render 38/38/42 instead
/// of 39/39/40).
pub fn is_equal_scale(scale: &[u16], n: usize) -> bool {
    if n <= 1 {
        return true;
    }
    if scale.len() != n - 1 {
        return false;
    }
    let sum: u32 = scale.iter().map(|p| *p as u32).sum();
    if sum > 100 {
        return false;
    }
    let last = 100 - sum;
    let mut shares: Vec<u32> = scale.iter().map(|p| *p as u32).collect();
    shares.push(last);
    let min = *shares.iter().min().unwrap_or(&0);
    let max = *shares.iter().max().unwrap_or(&0);
    max.saturating_sub(min) <= 1
}

/// The pane shares (summing to 100) of the scale, from index `from` to
/// the end (the implicit last share included).
pub fn shares_from(scale: &[u16], n: usize, from: usize) -> Vec<u32> {
    let _ = n;
    let sum: u32 = scale.iter().map(|p| *p as u32).sum();
    let last = (100 - sum).max(1);
    let mut v: Vec<u32> = scale
        .iter()
        .skip(from)
        .map(|p| (*p).max(1) as u32)
        .collect();
    v.push(last);
    v
}

/// Distribute `cells` across `shares` proportionally (largest-remainder,
/// ties → later index). Each pane gets ≥ 1 cell when `cells ≥
/// shares.len()`; the distribution sums to exactly `cells`.
pub fn distribute(cells: u16, shares: &[u32]) -> Vec<u16> {
    let count = shares.len();
    if count == 0 {
        return Vec::new();
    }
    if cells == 0 {
        return vec![0; count];
    }
    if cells < count as u16 {
        let mut v = vec![1u16; count];
        let mut left = cells;
        for s in v.iter_mut() {
            *s = (left > 0) as u16;
            left = left.saturating_sub(1);
        }
        return v;
    }
    let wsum: u32 = shares.iter().copied().sum::<u32>().max(count as u32);
    let nums: Vec<u32> = shares.iter().map(|s| s * cells as u32).collect();
    largest_remainder(&nums, wsum)
        .into_iter()
        .map(|s| s.max(1) as u16)
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn equal_scale_shapes() {
        assert_eq!(equal_scale(2), vec![50]);
        assert_eq!(equal_scale(3), vec![33, 33]);
        assert_eq!(equal_scale(4), vec![25, 25, 25]);
    }

    #[test]
    fn adding_a_pane_to_an_equal_split_stays_equal() {
        // 50/50 -> 3 panes: three thirds (last share 34 is implicit).
        assert_eq!(scale_scales(&[50], 3), vec![33, 33]);
        // 50/50 -> 4 panes: four quarters.
        assert_eq!(scale_scales(&[50], 4), vec![25, 25, 25]);
    }

    #[test]
    fn adding_a_pane_rescales_proportionally() {
        // 40/60 -> 3 panes: 40*2/3 = 26.67 (rem 2 gets the +1), 60*2/3 =
        // 40, new pane 100/3 = 33.33 → shares [27, 40, 33], entries [27, 40].
        assert_eq!(scale_scales(&[40], 3), vec![27, 40]);
        assert_eq!(100 - 27 - 40, 33, "the new pane's implicit share");
    }

    #[test]
    fn removing_a_pane_renormalizes_the_survivors() {
        // [40, 30] + implicit 30 -> 2 panes: survivors 40/30 (W' = 70) →
        // 57.14/42.86 → entries [57], last share 43.
        assert_eq!(scale_scales(&[40, 30], 2), vec![57]);
    }

    #[test]
    fn rescale_is_idempotent_for_equal_scales() {
        let eq = equal_scale(3);
        assert_eq!(scale_scales(&eq, 3), eq);
    }

    #[test]
    fn rescale_invariants_two_to_four_panes() {
        for old_n in 2..=4 {
            for new_n in 2..=4 {
                for old in [
                    equal_scale(old_n),
                    if old_n == 2 {
                        vec![40]
                    } else if old_n == 3 {
                        vec![40, 30]
                    } else {
                        vec![30, 20, 10]
                    },
                ] {
                    let out = scale_scales(&old, new_n);
                    assert_eq!(out.len(), new_n - 1, "n-1 entries ({old:?} -> {new_n})");
                    let sum: u32 = out.iter().map(|p| *p as u32).sum();
                    assert!(sum <= 99, "entries {out:?} leave the last share ≥ 1");
                    for p in &out {
                        assert!(*p >= 1, "every pane keeps a share: {out:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn drag_on_the_first_divider_reallocates_to_the_right() {
        // 3 panes at 33/33/34: dragging divider 0 to 64% shrinks panes 1
        // and 2 proportionally (18/18), pane 0 takes 64%.
        assert_eq!(set_drag_pcts(&[33, 33], 0, 64), vec![64, 18]);
        assert_eq!(100 - 64 - 18, 18, "the last share absorbs the remainder");
    }

    #[test]
    fn drag_on_the_last_divider_only_changes_the_last_share() {
        // 3 panes at 33/33/34: dragging divider 1 to 63% → entries
        // [33, 63], the last share drops to 4 (pane 0 untouched).
        assert_eq!(set_drag_pcts(&[33, 33], 1, 63), vec![33, 63]);
    }

    #[test]
    fn distribute_is_proportional_and_exact() {
        // 118 cells across [40, 30, 30] → 47/35/36 (sum 118).
        let v = distribute(118, &[40, 30, 30]);
        assert_eq!(v.iter().copied().sum::<u16>(), 118);
        assert_eq!(v, vec![47, 35, 36]);
        // Even split: 39/39/40 of 118.
        let v = distribute(118, &[1, 1, 1]);
        assert_eq!(v, vec![39, 39, 40]);
        // Degenerate: fewer cells than panes → 1-cell floor.
        assert_eq!(distribute(2, &[5, 5, 5]), vec![1, 1, 0]);
        assert_eq!(distribute(0, &[5, 5]), vec![0, 0]);
    }

    #[test]
    fn shares_from_includes_the_implicit_last_share() {
        assert_eq!(shares_from(&[40, 30], 3, 1), vec![30, 30]);
        assert_eq!(shares_from(&[33, 33], 3, 0), vec![33, 33, 34]);
    }

    #[test]
    fn normalize_handles_legacy_short_and_over_sums() {
        // Legacy empty scale on a 3-pane grid → equal.
        assert_eq!(normalize_scale(&[], 3), vec![33, 33]);
        // Legacy [40] (2-pane 40/60) on a 3-pane grid → proportional.
        assert_eq!(normalize_scale(&[40], 3), vec![27, 40]);
        // Over-sum hand-edit → renormalized, last share ≥ 1.
        // weights [90,90,1] → shares [50,50,0] → steal 1 from an entry:
        // [50, 49], last share 1.
        let out = normalize_scale(&[90, 90], 3);
        assert_eq!(out, vec![50, 49], "90/90 renormalizes to 50/49 + 1");
        assert_eq!(out.len(), 2);
        assert!(out.iter().map(|p| *p as u32).sum::<u32>() <= 99);
        // Full valid scale passes through unchanged.
        assert_eq!(normalize_scale(&[40, 30], 3), vec![40, 30]);
    }
}
