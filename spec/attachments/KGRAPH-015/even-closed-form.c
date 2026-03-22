#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Closed-form even-m rule (validated for even m >= 8).
 * Returns one of the 6 direction permutations as a string:
 *   "012", "021", "102", "120", "201", "210".
 */
static const char *d_layer_m2(int m, int i, int j) {
  int c = m / 2;
  int mod0 = (m % 4) == 0;

  /* Top overrides */
  if (i == 0) {
    if (j == m - 2) return "210";
    if (j == m - 1) return "102";
    return "012";
  }
  if (i == 1) {
    if (j == m - 2) return "201";
    if (j == m - 1) return "210";
    return "012";
  }

  /* Far zones: identity + one diagonal + one wall */
  if (i <= c - 3) {
    if (j == m - 1 - i) return "102";
    if (j == m - 1) return "210";
    return "012";
  }
  if (i >= c + 2) {
    if (j == m - 1 - i) return "102";
    if (j == m - 2) return "210";
    return "012";
  }

  /* Center band rows */
  if (i == c - 2) {
    if (j <= c) return "021";
    if (j == c + 1) return "120";
    if (j == m - 2) return mod0 ? "012" : "102";
    if (j == m - 1) return mod0 ? "201" : "021";
    return "012";
  }

  if (i == c - 1 || i == c) {
    int left = (i == c - 1) ? c - 1 : c - 2;
    int cusp = (i == c - 1) ? c : c - 1;
    int middle_lo = (i == c - 1) ? c + 1 : c;
    if (j <= left) return "021";
    if (j == cusp) return "120";
    if (j >= middle_lo && j <= m - 3) return "021";
    if (j == m - 2) return mod0 ? "021" : "201";
    if (j == m - 1) return mod0 ? "201" : "021";
    return "012";
  }

  /* i == c+1 */
  if (i == c + 1) {
    if (j <= c - 3) return "012";
    if (j == c - 2) return "102";
    if (j >= c - 1 && j <= m - 3) return "021";
    if (j == m - 2) return (mod0 ? "120" : "210");
    return "012";
  }

  return "012";
}

static const char *d_even_ge8(int m, int i, int j, int k) {
  int s = (i + j + k) % m;
  int c = m / 2;
  if (s <= m - 3) return "012";
  if (s == m - 1) return ((i == c - 1) || (i == c)) ? "210" : "120";
  return d_layer_m2(m, i, j); /* s == m-2 */
}

static int idx3(int m, int i, int j, int k) {
  return (i * m + j) * m + k;
}

static int verify_factor(const int *succ, int n, int factor) {
  int *indeg = (int *)calloc((size_t)n, sizeof(int));
  unsigned char *seen = (unsigned char *)calloc((size_t)n, sizeof(unsigned char));
  int ok = 1;
  int v, w;
  int cycles = 0;
  int max_cycle_len = 0;
  int visited = 0;

  if (indeg == NULL || seen == NULL) {
    fprintf(stderr, "verify: allocation failed\n");
    free(indeg);
    free(seen);
    return 0;
  }

  for (v = 0; v < n; v++) {
    w = succ[v];
    if (w < 0 || w >= n) {
      fprintf(stderr, "verify: factor %d has out-of-range successor at v=%d\n", factor, v);
      ok = 0;
      goto done;
    }
    indeg[w]++;
  }

  for (v = 0; v < n; v++) {
    if (indeg[v] != 1) {
      fprintf(stderr, "verify: factor %d indegree[%d]=%d (expected 1)\n", factor, v, indeg[v]);
      ok = 0;
      goto done;
    }
  }

  for (v = 0; v < n; v++) {
    int len = 0;
    int u;
    if (seen[v]) continue;
    cycles++;
    u = v;
    while (!seen[u]) {
      seen[u] = 1;
      len++;
      visited++;
      u = succ[u];
    }
    if (len > max_cycle_len) max_cycle_len = len;
  }

  if (cycles != 1 || max_cycle_len != n || visited != n) {
    fprintf(stderr,
            "verify: factor %d has cycles=%d, max-cycle-len=%d, visited=%d "
            "(expected 1 cycle of len %d, visited=%d)\n",
            factor, cycles, max_cycle_len, visited, n, n);
    ok = 0;
  }

done:
  free(indeg);
  free(seen);
  return ok;
}

static int sanity_check(int m) {
  int c, i, j, k, n, v;
  int *succ;
  int all_ok = 1;

  n = m * m * m;
  succ = (int *)malloc((size_t)(3 * n) * sizeof(int));
  if (succ == NULL) {
    fprintf(stderr, "verify: allocation failed\n");
    return 0;
  }

  for (i = 0; i < m; i++) {
    for (j = 0; j < m; j++) {
      for (k = 0; k < m; k++) {
        const char *d = d_even_ge8(m, i, j, k);
        v = idx3(m, i, j, k);
        for (c = 0; c < 3; c++) {
          if (d[c] == '0') succ[c * n + v] = idx3(m, (i + 1) % m, j, k);
          else if (d[c] == '1') succ[c * n + v] = idx3(m, i, (j + 1) % m, k);
          else succ[c * n + v] = idx3(m, i, j, (k + 1) % m);
        }
      }
    }
  }

  for (c = 0; c < 3; c++) {
    if (!verify_factor(&succ[c * n], n, c)) all_ok = 0;
  }

  free(succ);
  if (all_ok) {
    fprintf(stderr, "verify: OK (3 factors, each one cycle of length %d)\n", n);
  }
  return all_ok;
}

int main(int argc, char **argv) {
  int c, i, j, k, m, t, n;
  int do_check = 0;
  int have_m = 0;
  int argi;
  const char *d;

  for (argi = 1; argi < argc; argi++) {
    if (strcmp(argv[argi], "--check") == 0) {
      do_check = 1;
    } else if (!have_m) {
      m = atoi(argv[argi]);
      have_m = 1;
    } else {
      fprintf(stderr, "usage: %s m [--check]\n", argv[0]);
      return 1;
    }
  }

  if (!have_m) {
    fprintf(stderr, "usage: %s m [--check]\n", argv[0]);
    return 1;
  }

  if (m < 8 || (m % 2) != 0) {
    fprintf(stderr, "this closed-form C rule is for even m >= 8\n");
    return 1;
  }

  n = m * m * m;
  for (c = 0; c < 3; c++) {
    for (t = i = j = k = 0;; t++) {
      printf("%x%x%x ", i, j, k);
      if (t == n) break;
      d = d_even_ge8(m, i, j, k);

      switch (d[c]) {
        case '0': i = (i + 1) % m; break;
        case '1': j = (j + 1) % m; break;
        case '2': k = (k + 1) % m; break;
      }
    }
    printf("\n");
  }

  if (do_check && !sanity_check(m)) return 2;
  return 0;
}
