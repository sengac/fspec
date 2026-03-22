# -*- coding: utf-8 -*-
"""
Constructive Hamiltonian cycle decomposition of Cay(Z_m^3, {e_0,e_1,e_2})
for EVEN m >= 4.

The digraph has m^3 vertices (i,j,k) with 0 <= i,j,k < m, and three arcs
from each vertex:
  (i,j,k) -> ((i+1)%m, j, k)   direction 0  "R"
  (i,j,k) -> (i, (j+1)%m, k)   direction 1  "U"
  (i,j,k) -> (i, j, (k+1)%m)   direction 2  "I"

The construction partitions all 3*m^3 arcs into three directed Hamiltonian
cycles (each of length m^3).

CONSTRUCTION OVERVIEW:

The quotient map s = (i+j+k) mod m partitions vertices into m fibers.
At fiber position F = m//2, a "complex fiber" is placed.
The remaining m-1 fiber positions get "structured fibers" of three types:
  Type A: pair (RUI, URI) with threshold t_a
  Type B: pair (RIU, UIR) with threshold t_b
  Type C: pair (IRU, IUR) with threshold t_c

Each structured fiber at position s assigns permutation sigma(i,j):
  sigma(i,j) = special_perm  if (i+j) mod m == threshold
  sigma(i,j) = default_perm  otherwise

The structured fibers use (m-3) Type A + 1 Type B + 1 Type C.

The complex fiber at F=m//2 uses the "complex_fiber_v2" formula:
  Background: RUI
  URI staircase: at (0, m-1) and (i, m-i) for i=3..m-1
  RIU column: at (1, j) for j=1..m-3 and at (2, m-1)
  IUR row: at (i, 0) for i=2..m-1
  UIR corners: at (0, 0) and (1, m-2)
  IRU corners: at (1, 0) and (2, m-2)

The thresholds (t_a, t_b, t_c) are chosen to make all three 2D cycle
compositions Hamiltonian. For most m: t_a=m-1, t_b=0, t_c found by search
(at most m candidates). For m=4: a dedicated small construction is used.

Verified for all even m from 4 through 50.

The threshold search uses precomputed c1/c2 base permutations (O(m^2)) and
iterates only over c2-valid candidates (step 2 or 6), giving O(m) total
search work beyond the O(m^2) precomputation.
"""

import sys

# === Permutation constants (c0, c1, c2) ===
RUI = (0, 1, 2)
URI = (1, 0, 2)
RIU = (0, 2, 1)
UIR = (1, 2, 0)
IRU = (2, 0, 1)
IUR = (2, 1, 0)

PAIR_PERMS = {
    'A': (RUI, URI),  # default, special
    'B': (RIU, UIR),
    'C': (IRU, IUR),
}


def complex_fiber(m):
    """Construct the complex fiber at F = m//2.
    
    Returns dict: (i, j) -> (c0_dir, c1_dir, c2_dir)
    """
    fb = {}
    for i in range(m):
        for j in range(m):
            fb[(i, j)] = RUI

    # URI staircase
    fb[(0, m - 1)] = URI
    for i in range(3, m):
        fb[(i, m - i)] = URI

    # RIU column
    for j in range(1, m - 2):
        fb[(1, j)] = RIU
    fb[(2, m - 1)] = RIU

    # IUR row
    for i in range(2, m):
        fb[(i, 0)] = IUR

    # Corner pairs
    fb[(0, 0)] = UIR
    fb[(1, m - 2)] = UIR
    fb[(1, 0)] = IRU
    fb[(2, m - 2)] = IRU

    return fb


def build_structured_fiber(pair_type, threshold, m):
    """Build a structured fiber with given type and threshold."""
    default, special = PAIR_PERMS[pair_type]
    fb = {}
    for i in range(m):
        for j in range(m):
            d = (i + j) % m
            fb[(i, j)] = special if d == threshold else default
    return fb


def build_all_fibers(m, t_a, t_b, t_c):
    """Build all m fibers for the construction."""
    F = m // 2
    fibers_list = [s for s in range(m) if s != F]
    n = len(fibers_list)
    n_a = n - 2

    fibers = {}
    idx = 0
    for i in range(n_a):
        fibers[fibers_list[idx]] = build_structured_fiber('A', t_a, m)
        idx += 1
    fibers[fibers_list[idx]] = build_structured_fiber('B', t_b, m)
    idx += 1
    fibers[fibers_list[idx]] = build_structured_fiber('C', t_c, m)
    idx += 1

    fibers[F] = complex_fiber(m)
    return fibers


def fiber_action(fb, m, c):
    """Compute the permutation of Z_m^2 induced by fiber fb for cycle c."""
    perm = {}
    for i in range(m):
        for j in range(m):
            d = fb[(i, j)][c]
            if d == 0:
                perm[(i, j)] = ((i + 1) % m, j)
            elif d == 1:
                perm[(i, j)] = (i, (j + 1) % m)
            else:
                perm[(i, j)] = (i, j)
    return perm


def compose_perms(p1, p2):
    """Return p2 o p1 (apply p1 first, then p2)."""
    return {k: p2[p1[k]] for k in p1}


def count_cycles(perm):
    """Count the number of cycles in a permutation."""
    vis = set()
    n = 0
    for k in perm:
        if k not in vis:
            n += 1
            p = k
            while p not in vis:
                vis.add(p)
                p = perm[p]
    return n


def check_all_hc(fibers, m):
    """Check if all 3 cycle compositions are Hamiltonian (single cycle)."""
    for c in range(3):
        identity = {(i, j): (i, j) for i in range(m) for j in range(m)}
        comp = dict(identity)
        for s in range(m):
            act = fiber_action(fibers[s], m, c)
            comp = compose_perms(comp, act)
        if count_cycles(comp) != 1:
            return False
    return True


def _A_pow_k(m, k, t_a):
    """Compute A(t_a)^k directly in O(m^2).

    A(t_a) advances diagonal d->d+1, shifting i at d==t_a, j elsewhere.
    After k steps from (i,j) with d=(i+j)%m:
      i-shift = 1 iff (t_a - d) % m < k  (t_a is visited in the k steps)
      new diagonal = (d + k) % m
    """
    p = {}
    for i in range(m):
        for j in range(m):
            d = (i + j) % m
            shift = 1 if (t_a - d) % m < k else 0
            new_i = (i + shift) % m
            new_d = (d + k) % m
            new_j = (new_d - new_i) % m
            p[(i, j)] = (new_i, new_j)
    return p


def _serp_perm(threshold, m):
    """Serpentine permutation: i-shift (R) at d==threshold, j-shift (U) elsewhere."""
    p = {}
    for i in range(m):
        for j in range(m):
            d = (i + j) % m
            p[(i, j)] = ((i + 1) % m, j) if d == threshold else (i, (j + 1) % m)
    return p


def _serp_perm_c1(threshold, m):
    """Serpentine for c1: j-shift (U) at d==threshold, i-shift (R) elsewhere."""
    p = {}
    for i in range(m):
        for j in range(m):
            d = (i + j) % m
            p[(i, j)] = (i, (j + 1) % m) if d == threshold else ((i + 1) % m, j)
    return p


def _precompute_bases(m, t_b):
    """Precompute c1 and c2 base compositions (everything except C fiber) in O(m^2).

    Returns (base_c1, base_c2) such that:
      c1_comp(t_c) = C(t_c)_c1 o base_c1
      c2_comp(t_c) = C(t_c)_c2 o base_c2
    """
    t_a = m - 1
    F = m // 2

    fb_cx = complex_fiber(m)
    X_c1 = fiber_action(fb_cx, m, 1)
    X_c2 = fiber_action(fb_cx, m, 2)

    # c2 base = B(t_b)_c2 o X_c2  (X applied first, then B)
    base_c2 = compose_perms(X_c2, _serp_perm(t_b, m))

    # c1 base: fibers before X are A^F, after X are A^{m-3-F} then B (identity for c1)
    # base_c1 = A^{m-3-F} o X_c1 o A^F
    A_before = _A_pow_k(m, F, t_a)
    A_after = _A_pow_k(m, m - 3 - F, t_a)
    base_c1 = compose_perms(compose_perms(A_before, X_c1), A_after)

    return base_c1, base_c2


def find_thresholds(m):
    """Find working thresholds (t_a, t_b, t_c) for given even m >= 6.

    Uses precomputed c1/c2 bases (O(m^2)) and iterates only over c2-valid
    t_c candidates (step 2 or 6), giving O(m) search work beyond precomputation.
    Falls back to a full scan for the rare exceptional cases (m=8, 10, 40...).
    """
    # c2-valid t_c candidates: t_c ? offset (mod step)
    h = m // 2
    if h % 3 == 0:
        step, offset = 6, 0
    elif h % 3 == 2:
        step, offset = 6, 2
    else:
        step, offset = 2, 0

    for t_b in (0, 2):
        base_c1, base_c2 = _precompute_bases(m, t_b)
        for t_c in range(offset, m, step):
            # Fast c1 and c2 checks using precomputed bases
            Cc1 = _serp_perm_c1(t_c, m)
            if count_cycles(compose_perms(Cc1, base_c1)) != 1:
                continue
            Cc2 = _serp_perm(t_c, m)
            if count_cycles(compose_perms(Cc2, base_c2)) != 1:
                continue
            # c0 doesn't depend on t_c; verify the full solution once
            fibers = build_all_fibers(m, m - 1, t_b, t_c)
            if check_all_hc(fibers, m):
                return (m - 1, t_b, t_c)

    # Full fallback (should not be reached for m >= 6)
    for t_a in range(m - 1, -1, -1):
        for t_b in range(m):
            for t_c in range(m):
                fibers = build_all_fibers(m, t_a, t_b, t_c)
                if check_all_hc(fibers, m):
                    return (t_a, t_b, t_c)

    return None


# === m=4 special construction ===
# m=4 is too small for the complex_fiber formula (needs m >= 6).
# Use a hardcoded fiber set found by exhaustive search.

def build_m4_fibers():
    """Hardcoded construction for m=4."""
    m = 4
    fibers = {}
    fibers[0] = build_structured_fiber('A', 3, 4)
    fibers[1] = build_structured_fiber('B', 3, 4)
    fibers[3] = build_structured_fiber('C', 3, 4)

    # Complex fiber s=2 (hardcoded from verified solution)
    fb = {}
    for i in range(4):
        for j in range(4):
            fb[(i, j)] = RUI
    fb[(0, 0)] = URI; fb[(0, 1)] = RIU; fb[(0, 2)] = RIU; fb[(0, 3)] = UIR
    fb[(1, 0)] = IUR; fb[(1, 3)] = IRU
    fb[(2, 0)] = IUR; fb[(2, 2)] = URI
    fb[(3, 0)] = IRU; fb[(3, 1)] = UIR
    fibers[2] = fb
    return fibers


# === Main interface ===

def direction_at(i, j, k, m, cycle, fibers):
    """Return the direction (0, 1, or 2) for the given cycle at vertex (i,j,k)."""
    s = (i + j + k) % m
    return fibers[s][(i, j)][cycle]


def step(i, j, k, d, m):
    """Take one step in direction d from (i,j,k)."""
    if d == 0:
        return ((i + 1) % m, j, k)
    elif d == 1:
        return (i, (j + 1) % m, k)
    else:
        return (i, j, (k + 1) % m)


def generate_cycle(m, cycle, fibers):
    """Generate cycle, returning list of (vertex, direction) pairs."""
    path = []
    i, j, k = 0, 0, 0
    for _ in range(m ** 3):
        d = direction_at(i, j, k, m, cycle, fibers)
        path.append(((i, j, k), d))
        i, j, k = step(i, j, k, d, m)
    assert (i, j, k) == (0, 0, 0), f"Cycle {cycle} did not return to origin!"
    return path


def verify_decomposition(m, fibers):
    """Verify: 3 Hamiltonian cycles using all arcs exactly once."""
    all_arcs = set()
    for c in range(3):
        path = generate_cycle(m, c, fibers)
        vertices = [v for v, d in path]
        assert len(set(vertices)) == m ** 3, \
            f"Cycle {c}: not Hamiltonian ({len(set(vertices))} unique vertices)"
        for v, d in path:
            arc = (v, d)
            assert arc not in all_arcs, f"Cycle {c}: duplicate arc {v} dir {d}"
            all_arcs.add(arc)
    assert len(all_arcs) == 3 * m ** 3, \
        f"Not all arcs used: {len(all_arcs)} != {3 * m ** 3}"
    return True


def build_even_decomposition(m):
    """Build the Hamiltonian decomposition for even m >= 4.
    
    Returns dict of fibers: s -> {(i,j) -> (c0, c1, c2)}.
    """
    assert m >= 4 and m % 2 == 0, f"m must be even >= 4, got {m}"

    if m == 4:
        return build_m4_fibers()

    # m >= 6: use complex_fiber + threshold search
    thresholds = find_thresholds(m)
    assert thresholds is not None, f"No thresholds found for m={m}"
    t_a, t_b, t_c = thresholds
    return build_all_fibers(m, t_a, t_b, t_c)


def print_cycle(m, cycle, path):
    """Print a cycle in a compact format."""
    DIR_NAMES = {0: "i+1", 1: "j+1", 2: "k+1"}
    print(f"Cycle {cycle} (length {len(path)}):")
    for idx, ((i, j, k), d) in enumerate(path):
        if idx < 20 or idx >= len(path) - 5:
            print(f"  ({i},{j},{k}) -> {DIR_NAMES[d]}")
        elif idx == 20:
            print(f"  ... ({len(path) - 25} more steps) ...")


def main():
    if len(sys.argv) > 1:
        m = int(sys.argv[1])
    else:
        m = 8

    if m < 4 or m % 2 != 0:
        print(f"Error: m must be even >= 4, got {m}")
        sys.exit(1)

    if m == 2:
        print("m=2 is impossible (proven).")
        sys.exit(1)

    print(f"Constructing Hamiltonian decomposition for m={m} (even)...")
    fibers = build_even_decomposition(m)

    print("Verifying...")
    verify_decomposition(m, fibers)
    print(f"OK: 3 Hamiltonian cycles, {3 * m ** 3} arcs, all verified.")

    for c in range(3):
        path = generate_cycle(m, c, fibers)
        print()
        print_cycle(m, c, path)


if __name__ == "__main__":
    main()
