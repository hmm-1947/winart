package android.os;

import android.winrt.Native;

public final class SystemClock {
    private SystemClock() {}

    public static long uptimeMillis() {
        return Native.uptimeMillis();
    }

    public static long elapsedRealtime() {
        return Native.uptimeMillis();
    }

    public static long elapsedRealtimeNanos() {
        return Native.uptimeMillis() * 1000000L;
    }

    public static long currentThreadTimeMillis() {
        return Native.uptimeMillis();
    }

    public static void sleep(long ms) {
        boolean interrupted = false;
        long start = uptimeMillis();
        long left = ms;
        while (left > 0) {
            try {
                Thread.sleep(left);
                break;
            } catch (InterruptedException e) {
                interrupted = true;
                left = start + ms - uptimeMillis();
            }
        }
        if (interrupted) {
            Thread.currentThread().interrupt();
        }
    }
}
