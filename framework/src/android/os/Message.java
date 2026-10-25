package android.os;

public final class Message {
    public int what;
    public int arg1;
    public int arg2;
    public Object obj;
    Bundle data;
    long when;
    int flags;
    Handler target;
    Runnable callback;
    Message next;

    private static final Object sPoolSync = new Object();
    private static Message sPool;
    private static int sPoolSize = 0;
    private static final int MAX_POOL_SIZE = 50;
    static final int FLAG_IN_USE = 1;
    static final int FLAG_ASYNCHRONOUS = 2;

    public Message() {}

    public static Message obtain() {
        synchronized (sPoolSync) {
            if (sPool != null) {
                Message m = sPool;
                sPool = m.next;
                m.next = null;
                m.flags = 0;
                sPoolSize--;
                return m;
            }
        }
        return new Message();
    }

    public static Message obtain(Message orig) {
        Message m = obtain();
        m.what = orig.what;
        m.arg1 = orig.arg1;
        m.arg2 = orig.arg2;
        m.obj = orig.obj;
        m.target = orig.target;
        m.callback = orig.callback;
        if (orig.data != null) {
            m.data = new Bundle(orig.data);
        }
        return m;
    }

    public static Message obtain(Handler h) {
        Message m = obtain();
        m.target = h;
        return m;
    }

    public static Message obtain(Handler h, Runnable callback) {
        Message m = obtain();
        m.target = h;
        m.callback = callback;
        return m;
    }

    public static Message obtain(Handler h, int what) {
        Message m = obtain();
        m.target = h;
        m.what = what;
        return m;
    }

    public static Message obtain(Handler h, int what, Object obj) {
        Message m = obtain();
        m.target = h;
        m.what = what;
        m.obj = obj;
        return m;
    }

    public static Message obtain(Handler h, int what, int arg1, int arg2) {
        Message m = obtain();
        m.target = h;
        m.what = what;
        m.arg1 = arg1;
        m.arg2 = arg2;
        return m;
    }

    public static Message obtain(Handler h, int what, int arg1, int arg2, Object obj) {
        Message m = obtain();
        m.target = h;
        m.what = what;
        m.arg1 = arg1;
        m.arg2 = arg2;
        m.obj = obj;
        return m;
    }

    public void recycle() {
        if (isInUse()) {
            throw new IllegalStateException("This message cannot be recycled because it is still in use.");
        }
        recycleUnchecked();
    }

    void recycleUnchecked() {
        flags = FLAG_IN_USE;
        what = 0;
        arg1 = 0;
        arg2 = 0;
        obj = null;
        when = 0;
        target = null;
        callback = null;
        data = null;
        synchronized (sPoolSync) {
            if (sPoolSize < MAX_POOL_SIZE) {
                next = sPool;
                sPool = this;
                sPoolSize++;
            }
        }
    }

    public void copyFrom(Message o) {
        flags = o.flags & ~FLAG_IN_USE;
        what = o.what;
        arg1 = o.arg1;
        arg2 = o.arg2;
        obj = o.obj;
        if (o.data != null) {
            data = new Bundle(o.data);
        } else {
            data = null;
        }
    }

    public long getWhen() {
        return when;
    }

    public void setTarget(Handler t) {
        target = t;
    }

    public Handler getTarget() {
        return target;
    }

    public Runnable getCallback() {
        return callback;
    }

    public Message setCallback(Runnable r) {
        callback = r;
        return this;
    }

    public Bundle getData() {
        if (data == null) {
            data = new Bundle();
        }
        return data;
    }

    public Bundle peekData() {
        return data;
    }

    public void setData(Bundle b) {
        data = b;
    }

    public Message setWhat(int w) {
        what = w;
        return this;
    }

    public void sendToTarget() {
        target.sendMessage(this);
    }

    public boolean isAsynchronous() {
        return (flags & FLAG_ASYNCHRONOUS) != 0;
    }

    public void setAsynchronous(boolean async) {
        if (async) {
            flags |= FLAG_ASYNCHRONOUS;
        } else {
            flags &= ~FLAG_ASYNCHRONOUS;
        }
    }

    boolean isInUse() {
        return (flags & FLAG_IN_USE) == FLAG_IN_USE;
    }

    void markInUse() {
        flags |= FLAG_IN_USE;
    }

    public String toString() {
        StringBuilder b = new StringBuilder();
        b.append("{ when=").append(when - SystemClock.uptimeMillis());
        if (target != null) {
            if (callback != null) {
                b.append(" callback=").append(callback.getClass().getName());
            } else {
                b.append(" what=").append(what);
            }
            if (arg1 != 0) {
                b.append(" arg1=").append(arg1);
            }
            if (arg2 != 0) {
                b.append(" arg2=").append(arg2);
            }
            if (obj != null) {
                b.append(" obj=").append(obj);
            }
        } else {
            b.append(" barrier");
        }
        b.append(" }");
        return b.toString();
    }
}
