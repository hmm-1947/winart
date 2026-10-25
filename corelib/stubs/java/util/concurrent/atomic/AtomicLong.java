package java.util.concurrent.atomic;

import java.util.function.LongBinaryOperator;
import java.util.function.LongUnaryOperator;

public class AtomicLong extends Number implements java.io.Serializable {
    private static final long serialVersionUID = 1927816293512124184L;
    static final boolean VM_SUPPORTS_LONG_CAS = true;
    private volatile long value;

    public AtomicLong(long initialValue) { value = initialValue; }
    public AtomicLong() { }
    public final long get() { return value; }
    public final void set(long v) { value = v; }
    public final void lazySet(long v) { value = v; }
    public final long getAndSet(long v) { long o = value; value = v; return o; }
    public final boolean compareAndSet(long e, long v) { if (value == e) { value = v; return true; } return false; }
    public final boolean weakCompareAndSet(long e, long v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetPlain(long e, long v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetVolatile(long e, long v) { return compareAndSet(e, v); }
    public final long getAndIncrement() { return value++; }
    public final long getAndDecrement() { return value--; }
    public final long getAndAdd(long d) { long o = value; value = o + d; return o; }
    public final long incrementAndGet() { return ++value; }
    public final long decrementAndGet() { return --value; }
    public final long addAndGet(long d) { return value += d; }
    public final long getAndUpdate(LongUnaryOperator f) { long o = value; value = f.applyAsLong(o); return o; }
    public final long updateAndGet(LongUnaryOperator f) { return value = f.applyAsLong(value); }
    public final long getAndAccumulate(long x, LongBinaryOperator f) { long o = value; value = f.applyAsLong(o, x); return o; }
    public final long accumulateAndGet(long x, LongBinaryOperator f) { return value = f.applyAsLong(value, x); }
    public String toString() { return Long.toString(value); }
    public int intValue() { return (int) value; }
    public long longValue() { return value; }
    public float floatValue() { return (float) value; }
    public double doubleValue() { return (double) value; }
    public final long getPlain() { return value; }
    public final void setPlain(long v) { value = v; }
    public final long getOpaque() { return value; }
    public final void setOpaque(long v) { value = v; }
    public final long getAcquire() { return value; }
    public final void setRelease(long v) { value = v; }
    public final long compareAndExchange(long e, long v) { long o = value; if (o == e) value = v; return o; }
    public final long compareAndExchangeAcquire(long e, long v) { return compareAndExchange(e, v); }
    public final long compareAndExchangeRelease(long e, long v) { return compareAndExchange(e, v); }
    public final boolean weakCompareAndSetAcquire(long e, long v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetRelease(long e, long v) { return compareAndSet(e, v); }
}
