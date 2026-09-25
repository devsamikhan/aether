#include <iostream>
#include <chrono>

struct Node {
    int val;
    int tag;
};

int main() {
    const int N = 500000;
    auto t0 = std::chrono::high_resolution_clock::now();
    long long sum = 0;
    for (int i = 0; i < N; ++i) {
        Node* node = new Node{i, 1};
        sum += node->val;
        delete node;
    }
    auto t1 = std::chrono::high_resolution_clock::now();
    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
    std::cout << "C++ Heap Allocation Churn (500k Nodes):" << std::endl;
    std::cout << "  Sum: " << sum << std::endl;
    std::cout << "  Time: " << ms << " ms" << std::endl;
    std::cout << "  Rate: " << (long long)(N / (ms / 1000.0)) << " allocations/sec" << std::endl;
    return 0;
}
