import time

N = 10000000
# In Python, creating 3 lists of 10M floats would take several GB of RAM and tens of seconds
# Let's test with 1M floats or array/streaming to see the scaling
print("Allocating and computing in Python...")
t0 = time.perf_counter()
# Using array or list
try:
    import array
    a = array.array('f', [0.0]) * N
    b = array.array('f', [0.0]) * N
    c = array.array('f', [0.0]) * N
    out = array.array('f', [0.0]) * N
    t_alloc = time.perf_counter()
    print(f"Python 10M allocation: {(t_alloc - t0)*1000.0:.2f} ms")
    
    t_comp0 = time.perf_counter()
    # Chunked computation to prevent total hang
    # Let's measure first 1M items and extrapolate or run all if fast
    total = 0.0
    for i in range(1000000): # 1 Million items test
        out[i] = a[i] * b[i] + c[i]
        total += out[i]
    t_comp1 = time.perf_counter()
    ms_1m = (t_comp1 - t_comp0) * 1000.0
    print(f"Python 1 Million items compute: {ms_1m:.2f} ms")
    print(f"Python 10 Million extrapolated: {ms_1m * 10.0:.2f} ms")
except Exception as e:
    print("Error:", e)
