package java.util.concurrent.atomic;

import java.util.function.BinaryOperator;
import java.util.function.UnaryOperator;

public class AtomicReference<V> implements java.io.Serializable {
    private static final long serialVersionUID = -1848883965231344442L;
    private volatile V value;

    public AtomicReference(V initialValue) { value = initialValue; }
    public AtomicReference() { }
    public final V get() { return value; }
    public final void set(V v) { value = v; }
    public final void lazySet(V v) { value = v; }
    public final boolean compareAndSet(V e, V v) { if (value == e) { value = v; return true; } return false; }
    public final boolean weakCompareAndSet(V e, V v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetPlain(V e, V v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetVolatile(V e, V v) { return compareAndSet(e, v); }
    public final V getAndSet(V v) { V o = value; value = v; return o; }
    public final V getAndUpdate(UnaryOperator<V> f) { V o = value; value = f.apply(o); return o; }
    public final V updateAndGet(UnaryOperator<V> f) { return value = f.apply(value); }
    public final V getAndAccumulate(V x, BinaryOperator<V> f) { V o = value; value = f.apply(o, x); return o; }
    public final V accumulateAndGet(V x, BinaryOperator<V> f) { return value = f.apply(value, x); }
    public String toString() { return String.valueOf(value); }
    public final V getPlain() { return value; }
    public final void setPlain(V v) { value = v; }
    public final V getOpaque() { return value; }
    public final void setOpaque(V v) { value = v; }
    public final V getAcquire() { return value; }
    public final void setRelease(V v) { value = v; }
    public final V compareAndExchange(V e, V v) { V o = value; if (o == e) value = v; return o; }
    public final boolean weakCompareAndSetAcquire(V e, V v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetRelease(V e, V v) { return compareAndSet(e, v); }
}
