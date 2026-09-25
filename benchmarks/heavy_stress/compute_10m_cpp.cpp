#include <iostream>
#include <vector>
#include <chrono>

int main() {
    const int N = 10000000;
    std::vector<float> a(N, 0.0f);
    std::vector<float> b(N, 0.0f);
    std::vector<float> c(N, 0.0f);
    std::vector<float> out(N, 0.0f);

    auto t0 = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < N; ++i) {
        out[i] = a[i] * b[i] + c[i];
    }
    double sum = 0.0;
    for (int i = 0; i < N; ++i) {
        sum += out[i];
    }
    auto t1 = std::chrono::high_resolution_clock::now();
    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();

    std::cout << "C++ 10 Million Floats FMA + Sum Time: " << ms << " ms (sum=" << sum << ")" << std::endl;
    return 0;
}
