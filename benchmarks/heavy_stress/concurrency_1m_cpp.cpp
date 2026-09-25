#include <windows.h>
#include <iostream>
#include <queue>
#include <chrono>

const int N = 1000000;
std::queue<int> q;
CRITICAL_SECTION cs;
HANDLE hSemProduce;
HANDLE hSemConsume;

DWORD WINAPI Producer(LPVOID lpParam) {
    for (int i = 0; i < N; ++i) {
        WaitForSingleObject(hSemProduce, INFINITE);
        EnterCriticalSection(&cs);
        q.push(i);
        LeaveCriticalSection(&cs);
        ReleaseSemaphore(hSemConsume, 1, NULL);
    }
    return 0;
}

int main() {
    InitializeCriticalSection(&cs);
    hSemProduce = CreateSemaphore(NULL, 1024, 1024, NULL);
    hSemConsume = CreateSemaphore(NULL, 0, 1024, NULL);

    auto t0 = std::chrono::high_resolution_clock::now();
    HANDLE hThread = CreateThread(NULL, 0, Producer, NULL, 0, NULL);

    long long sum = 0;
    for (int i = 0; i < N; ++i) {
        WaitForSingleObject(hSemConsume, INFINITE);
        EnterCriticalSection(&cs);
        int val = q.front();
        q.pop();
        LeaveCriticalSection(&cs);
        ReleaseSemaphore(hSemProduce, 1, NULL);
        sum += val;
    }

    WaitForSingleObject(hThread, INFINITE);
    CloseHandle(hThread);
    CloseHandle(hSemProduce);
    CloseHandle(hSemConsume);
    DeleteCriticalSection(&cs);

    auto t1 = std::chrono::high_resolution_clock::now();
    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
    std::cout << "C++ 1 Million Concurrency Time: " << ms << " ms" << std::endl;
    return 0;
}
