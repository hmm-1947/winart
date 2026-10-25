package android.os;

import android.winrt.Native;

public final class MessageQueue {
    Message mMessages;
    private boolean mQuitting;
    private final boolean mQuitAllowed;
    boolean mMain;

    MessageQueue(boolean quitAllowed) {
        mQuitAllowed = quitAllowed;
    }

    boolean enqueueMessage(Message msg, long when) {
        if (msg.target == null) {
            throw new IllegalArgumentException("Message must have a target.");
        }
        synchronized (this) {
            if (msg.isInUse()) {
                throw new IllegalStateException(msg + " This message is already in use.");
            }
            if (mQuitting) {
                msg.recycleUnchecked();
                return false;
            }
            msg.markInUse();
            msg.when = when;
            Message p = mMessages;
            if (p == null || when == 0 || when < p.when) {
                msg.next = p;
                mMessages = msg;
            } else {
                Message prev;
                for (;;) {
                    prev = p;
                    p = p.next;
                    if (p == null || when < p.when) {
                        break;
                    }
                }
                msg.next = p;
                prev.next = msg;
            }
            notifyAll();
        }
        if (mMain) {
            Native.wakeMain();
        }
        return true;
    }

    Message next() {
        for (;;) {
            synchronized (this) {
                long now = SystemClock.uptimeMillis();
                Message m = mMessages;
                long delay = 0;
                if (m != null) {
                    if (now >= m.when) {
                        mMessages = m.next;
                        m.next = null;
                        return m;
                    }
                    delay = m.when - now;
                } else if (mQuitting) {
                    return null;
                }
                try {
                    wait(delay);
                } catch (InterruptedException e) {
                }
            }
        }
    }

    Message pollDue() {
        synchronized (this) {
            Message m = mMessages;
            if (m != null && SystemClock.uptimeMillis() >= m.when) {
                mMessages = m.next;
                m.next = null;
                return m;
            }
            return null;
        }
    }

    long delayToNext() {
        synchronized (this) {
            Message m = mMessages;
            if (m == null) {
                return -1;
            }
            long d = m.when - SystemClock.uptimeMillis();
            return d < 0 ? 0 : d;
        }
    }

    void quit(boolean safe) {
        if (!mQuitAllowed) {
            throw new IllegalStateException("Main thread not allowed to quit.");
        }
        synchronized (this) {
            if (mQuitting) {
                return;
            }
            mQuitting = true;
            if (safe) {
                removeAllFutureMessagesLocked();
            } else {
                removeAllMessagesLocked();
            }
            notifyAll();
        }
        if (mMain) {
            Native.wakeMain();
        }
    }

    boolean isQuitting() {
        synchronized (this) {
            return mQuitting;
        }
    }

    public boolean isIdle() {
        synchronized (this) {
            Message m = mMessages;
            return m == null || SystemClock.uptimeMillis() < m.when;
        }
    }

    boolean hasMessages(Handler h, int what, Object object) {
        if (h == null) {
            return false;
        }
        synchronized (this) {
            for (Message p = mMessages; p != null; p = p.next) {
                if (p.target == h && p.what == what && (object == null || p.obj == object)) {
                    return true;
                }
            }
            return false;
        }
    }

    boolean hasCallbacks(Handler h, Runnable r, Object object) {
        if (h == null) {
            return false;
        }
        synchronized (this) {
            for (Message p = mMessages; p != null; p = p.next) {
                if (p.target == h && (r == null || p.callback == r) && (object == null || p.obj == object)) {
                    return true;
                }
            }
            return false;
        }
    }

    void removeMessages(Handler h, int what, Object object) {
        if (h == null) {
            return;
        }
        synchronized (this) {
            Message p = mMessages;
            while (p != null && p.target == h && p.what == what && (object == null || p.obj == object)) {
                Message n = p.next;
                mMessages = n;
                p.recycleUnchecked();
                p = n;
            }
            while (p != null) {
                Message n = p.next;
                if (n != null && n.target == h && n.what == what && (object == null || n.obj == object)) {
                    Message nn = n.next;
                    n.recycleUnchecked();
                    p.next = nn;
                    continue;
                }
                p = n;
            }
        }
    }

    void removeCallbacks(Handler h, Runnable r, Object object) {
        if (h == null || r == null) {
            return;
        }
        synchronized (this) {
            Message p = mMessages;
            while (p != null && p.target == h && p.callback == r && (object == null || p.obj == object)) {
                Message n = p.next;
                mMessages = n;
                p.recycleUnchecked();
                p = n;
            }
            while (p != null) {
                Message n = p.next;
                if (n != null && n.target == h && n.callback == r && (object == null || n.obj == object)) {
                    Message nn = n.next;
                    n.recycleUnchecked();
                    p.next = nn;
                    continue;
                }
                p = n;
            }
        }
    }

    void removeCallbacksAndMessages(Handler h, Object object) {
        if (h == null) {
            return;
        }
        synchronized (this) {
            Message p = mMessages;
            while (p != null && p.target == h && (object == null || p.obj == object)) {
                Message n = p.next;
                mMessages = n;
                p.recycleUnchecked();
                p = n;
            }
            while (p != null) {
                Message n = p.next;
                if (n != null && n.target == h && (object == null || n.obj == object)) {
                    Message nn = n.next;
                    n.recycleUnchecked();
                    p.next = nn;
                    continue;
                }
                p = n;
            }
        }
    }

    private void removeAllMessagesLocked() {
        Message p = mMessages;
        while (p != null) {
            Message n = p.next;
            p.recycleUnchecked();
            p = n;
        }
        mMessages = null;
    }

    private void removeAllFutureMessagesLocked() {
        long now = SystemClock.uptimeMillis();
        Message p = mMessages;
        if (p == null) {
            return;
        }
        if (p.when > now) {
            removeAllMessagesLocked();
            return;
        }
        Message n;
        for (;;) {
            n = p.next;
            if (n == null) {
                return;
            }
            if (n.when > now) {
                break;
            }
            p = n;
        }
        p.next = null;
        do {
            p = n;
            n = p.next;
            p.recycleUnchecked();
        } while (n != null);
    }
}
