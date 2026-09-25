#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <time.h>

int main() {
    int64_t size = 1000000;
    printf("[C GCC -O3] Processing contiguous buffer (%lld elements)...\n", size);

    clock_t start = clock();
    int64_t *buf = (int64_t *)calloc(size, sizeof(int64_t));
    for (int64_t i = 0; i < size; i++) {
        buf[i] = (i * 7) % 1000;
    }

    int64_t total = 0;
    for (int64_t i = 0; i < size; i++) {
        total += buf[i];
    }
    free(buf);
    clock_t end = clock();

    double duration_ms = ((double)(end - start) / CLOCKS_PER_SEC) * 1000.0;
    printf("Total: %lld\n", total);
    printf("Time: %.3f ms\n", duration_ms);

    return 0;
}
