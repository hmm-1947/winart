package java.util.concurrent.atomic;

public class AtomicBoolean implements java.io.Serializable {
    private static final long serialVersionUID = 4654671469794556979L;
    private volatile boolean value;

    public AtomicBoolean(boolean initialValue) { value = initialValue; }
    public AtomicBoolean() { }
    public final boolean get() { return value; }
    public final boolean compareAndSet(boolean e, boolean v) { if (value == e) { value = v; return true; } return false; }
    public boolean weakCompareAndSet(boolean e, boolean v) { return compareAndSet(e, v); }
    public boolean weakCompareAndSetPlain(boolean e, boolean v) { return compareAndSet(e, v); }
    public boolean weakCompareAndSetVolatile(boolean e, boolean v) { return compareAndSet(e, v); }
    public final void set(boolean v) { value = v; }
    public final void lazySet(boolean v) { value = v; }
    public final boolean getAndSet(boolean v) { boolean o = value; value = v; return o; }
    public String toString() { return Boolean.toString(value); }
    public final boolean getPlain() { return value; }
    public final void setPlain(boolean v) { value = v; }
    public final boolean getOpaque() { return value; }
    public final void setOpaque(boolean v) { value = v; }
    public final boolean getAcquire() { return value; }
    public final void setRelease(boolean v) { value = v; }
    public final boolean compareAndExchange(boolean e, boolean v) { boolean o = value; if (o == e) value = v; return o; }
    public final boolean weakCompareAndSetAcquire(boolean e, boolean v) { return compareAndSet(e, v); }
    public final boolean weakCompareAndSetRelease(boolean e, boolean v) { return compareAndSet(e, v); }
}
