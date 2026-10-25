package java.util.concurrent.atomic;

import java.util.function.IntBinaryOperator;
import java.util.function.IntUnaryOperator;

public class AtomicInteger extends Number implements java.io.Serializable {
    private static final long serialVersionUID = 6214790243416807050L;
    private volatile int value;

    public AtomicInteger(int initialValue) { value = initialValue; }
    public AtomicInteger() { }
    public final int get() { return value; }
    public final void set(int v) { value = v; }
    public final void lazySet(int v) { value = v; }
    public final int getAndSet(int v) { int o = value; value = v; return o; }
    public final boolean compareAndSet(int e, int v) { if (value == e) { value = v; return true; } return false; }
    public final boolean weakCompareAndSet(int e, int v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetPlain(int e, int v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetVolatile(int e, int v) { return compareAndSet(e, v); }
    public final int getAndIncrement() { return value++; }
    public final int getAndDecrement() { return value--; }
    public final int getAndAdd(int d) { int o = value; value = o + d; return o; }
    public final int incrementAndGet() { return ++value; }
    public final int decrementAndGet() { return --value; }
    public final int addAndGet(int d) { return value += d; }
    public final int getAndUpdate(IntUnaryOperator f) { int o = value; value = f.applyAsInt(o); return o; }
    public final int updateAndGet(IntUnaryOperator f) { return value = f.applyAsInt(value); }
    public final int getAndAccumulate(int x, IntBinaryOperator f) { int o = value; value = f.applyAsInt(o, x); return o; }
    public final int accumulateAndGet(int x, IntBinaryOperator f) { return value = f.applyAsInt(value, x); }
    public String toString() { return Integer.toString(value); }
    public int intValue() { return value; }
    public long longValue() { return (long) value; }
    public float floatValue() { return (float) value; }
    public double doubleValue() { return (double) value; }
    public final int getPlain() { return value; }
    public final void setPlain(int v) { value = v; }
    public final int getOpaque() { return value; }
    public final void setOpaque(int v) { value = v; }
    public final int getAcquire() { return value; }
    public final void setRelease(int v) { value = v; }
    public final int compareAndExchange(int e, int v) { int o = value; if (o == e) value = v; return o; }
    public final int compareAndExchangeAcquire(int e, int v) { return compareAndExchange(e, v); }
    public final int compareAndExchangeRelease(int e, int v) { return compareAndExchange(e, v); }
    public final boolean weakCompareAndSetAcquire(int e, int v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetRelease(int e, int v) { return compareAndSet(e, v); }
}
