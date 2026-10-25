package android.os;

import android.winrt.Native;

public final class Looper {
    private static final ThreadLocal<Looper> sThreadLocal = new ThreadLocal<Looper>();
    private static Looper sMainLooper;

    final MessageQueue mQueue;
    final Thread mThread;

    private Looper(boolean quitAllowed) {
        mQueue = new MessageQueue(quitAllowed);
        mThread = Thread.currentThread();
    }

    public static void prepare() {
        prepare(true);
    }

    private static void prepare(boolean quitAllowed) {
        if (sThreadLocal.get() != null) {
            throw new RuntimeException("Only one Looper may be created per thread");
        }
        sThreadLocal.set(new Looper(quitAllowed));
    }

    public static void prepareMainLooper() {
        prepare(false);
        synchronized (Looper.class) {
            if (sMainLooper != null) {
                throw new IllegalStateException("The main Looper has already been prepared.");
            }
            sMainLooper = myLooper();
            sMainLooper.mQueue.mMain = true;
        }
    }

    public static Looper getMainLooper() {
        synchronized (Looper.class) {
            return sMainLooper;
        }
    }

    public static Looper myLooper() {
        return sThreadLocal.get();
    }

    public static MessageQueue myQueue() {
        return sThreadLocal.get().mQueue;
    }

    public static void loop() {
        loopImpl();
    }

    public static long winrtPump() {
        Looper l = getMainLooper();
        if (l == null) {
            return -1;
        }
        MessageQueue q = l.mQueue;
        for (int i = 0; i < 32; i++) {
            Message m = q.pollDue();
            if (m == null) {
                return q.delayToNext();
            }
            try {
                m.target.dispatchMessage(m);
            } catch (Throwable t) {
                Native.log("uncaught exception on main thread: " + t);
            }
            m.recycleUnchecked();
        }
        return 0;
    }

    private static void loopImpl() {
        Looper me = myLooper();
        if (me == null) {
            throw new RuntimeException("No Looper; Looper.prepare() wasn't called on this thread.");
        }
        if (me == getMainLooper()) {
            throw new IllegalStateException("The main looper is driven by the window message loop.");
        }
        MessageQueue queue = me.mQueue;
        for (;;) {
            Message msg = queue.next();
            if (msg == null) {
                return;
            }
            try {
                msg.target.dispatchMessage(msg);
            } finally {
                msg.recycleUnchecked();
            }
        }
    }

    public boolean isCurrentThread() {
        return Thread.currentThread() == mThread;
    }

    public void quit() {
        mQueue.quit(false);
    }

    public void quitSafely() {
        mQueue.quit(true);
    }

    public Thread getThread() {
        return mThread;
    }

    public MessageQueue getQueue() {
        return mQueue;
    }

    public String toString() {
        return "Looper (" + mThread.getName() + ", tid " + mThread.getId() + ") {" + Integer.toHexString(System.identityHashCode(this)) + "}";
    }
}
