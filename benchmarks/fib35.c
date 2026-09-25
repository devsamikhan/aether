#include <stdio.h>
#include <stdint.h>
#include <time.h>

int64_t fib(int64_t n) {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

int main() {
    int64_t target = 35;
    printf("[C GCC -O3] Computing Fibonacci(%lld) recursively...\n", target);

    clock_t start = clock();
    int64_t result = fib(target);
    clock_t end = clock();

    double duration_ms = ((double)(end - start) / CLOCKS_PER_SEC) * 1000.0;
    printf("Result: %lld\n", result);
    printf("Time: %.3f ms\n", duration_ms);

    return 0;
}
