#include <stdio.h>
#include <stdint.h>
#include <time.h>

int main() {
    int64_t n = 10000000;
    printf("[C GCC -O3] Running intensive loop (%lld iterations)...\n", n);

    clock_t start = clock();
    int64_t total = 0;
    for (int64_t i = 0; i < n; i++) {
        total = total + (i * 3) - (i / 2);
    }
    clock_t end = clock();

    double duration_ms = ((double)(end - start) / CLOCKS_PER_SEC) * 1000.0;
    printf("Total: %lld\n", total);
    printf("Time: %.3f ms\n", duration_ms);

    return 0;
}
