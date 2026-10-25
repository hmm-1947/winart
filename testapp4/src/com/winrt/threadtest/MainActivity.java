package com.winrt.threadtest;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Message;
import android.os.SystemClock;
import android.widget.TextView;

public class MainActivity extends Activity {
    private static final String EXPECTED = "create front p1 p2 p3 m1 m2 m4 m5b stage2 echo7 echoRun echo8 d300 bg500 d900";

    private TextView tv;
    private final StringBuffer mainLog = new StringBuffer();
    private final StringBuffer failures = new StringBuffer();
    private final Object lock = new Object();
    private Handler mainHandler;
    private Handler bgHandler2;
    private Handler bgHandler3;
    private Thread mainThread;
    private Thread t1;
    private Thread t2;
    private Thread t3;
    private Object tokenA = new Object();
    private Object tokenB = new Object();
    private volatile boolean wrongThreadThrown;
    private volatile boolean bgIsNotMain;
    private volatile boolean bgNoLooper;
    private volatile boolean bgSawMainLooper;
    private volatile boolean noLooperHandlerThrows;
    private volatile boolean dupPrepareThrows;
    private volatile boolean t2CallbacksOnT2 = true;
    private volatile boolean t3Late;
    private volatile boolean allOnMain = true;
    private long t0;
    private long d300At;
    private boolean done;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        tv = new TextView(this);
        setContentView(tv);
        mainThread = Thread.currentThread();
        t0 = SystemClock.uptimeMillis();
        check("main-looper-exists", Looper.getMainLooper() != null);
        check("my-looper-is-main", Looper.myLooper() == Looper.getMainLooper());
        check("main-looper-thread", Looper.getMainLooper().getThread() == mainThread);
        check("main-thread-name", "main".equals(mainThread.getName()));
        check("main-thread-alive", mainThread.isAlive());
        check("main-thread-runnable", mainThread.getState() == Thread.State.RUNNABLE);
        check("is-current-thread", Looper.getMainLooper().isCurrentThread());
        step("create");
        mainHandler = new Handler(new Handler.Callback() {
            public boolean handleMessage(Message m) {
                if (m.what == 1) {
                    step("m1");
                    return true;
                }
                if (m.what == 2) {
                    step("m2");
                    return true;
                }
                if (m.what == 3) {
                    step("m3-bad");
                    return true;
                }
                if (m.what == 4) {
                    step("m4");
                    return true;
                }
                if (m.what == 5) {
                    step(m.obj == tokenB ? "m5b" : "m5a-bad");
                    return true;
                }
                return false;
            }
        });
        check("handler-looper", mainHandler.getLooper() == Looper.getMainLooper());
        mainHandler.post(r("p1"));
        mainHandler.post(r("p2"));
        mainHandler.post(r("p3"));
        check("post-not-synchronous", mainLog.toString().equals("create"));
        mainHandler.postDelayed(new Runnable() {
            public void run() {
                step("d300");
                d300At = SystemClock.uptimeMillis();
            }
        }, 300);
        mainHandler.postDelayed(new Runnable() {
            public void run() {
                step("d900");
                conclude();
            }
        }, 900);
        mainHandler.sendMessage(Message.obtain(mainHandler, 1));
        mainHandler.sendEmptyMessage(2);
        Runnable rc = r("rc-bad");
        mainHandler.post(rc);
        check("has-callbacks", mainHandler.hasCallbacks(rc));
        mainHandler.removeCallbacks(rc);
        check("callbacks-removed", !mainHandler.hasCallbacks(rc));
        mainHandler.sendEmptyMessage(3);
        mainHandler.sendEmptyMessage(3);
        check("has-messages-3", mainHandler.hasMessages(3));
        mainHandler.removeMessages(3);
        check("messages-3-removed", !mainHandler.hasMessages(3));
        mainHandler.sendEmptyMessage(4);
        check("has-messages-4", mainHandler.hasMessages(4));
        mainHandler.sendMessage(Message.obtain(mainHandler, 5, tokenA));
        mainHandler.sendMessage(Message.obtain(mainHandler, 5, tokenB));
        mainHandler.removeMessages(5, tokenA);
        check("messages-5a-removed", !mainHandler.hasMessages(5, tokenA));
        check("messages-5b-kept", mainHandler.hasMessages(5, tokenB));
        Object token = new Object();
        mainHandler.postDelayed(r("token-bad"), token, 20);
        mainHandler.removeCallbacksAndMessages(token);
        mainHandler.postAtFrontOfQueue(r("front"));
        mainHandler.post(new Runnable() {
            public void run() {
                step("stage2");
                stage2();
            }
        });
        mainHandler.postDelayed(new Runnable() {
            public void run() {
                finishWith("timeout");
            }
        }, 6000);
    }

    private void check(String name, boolean ok) {
        if (!ok) {
            failures.append(' ').append(name);
        }
    }

    private void step(String tag) {
        if (mainLog.length() > 0) {
            mainLog.append(' ');
        }
        mainLog.append(tag);
        if (Looper.myLooper() != Looper.getMainLooper() || Thread.currentThread() != mainThread) {
            allOnMain = false;
        }
    }

    private Runnable r(final String tag) {
        return new Runnable() {
            public void run() {
                step(tag);
            }
        };
    }

    private void waitReady(int which) {
        synchronized (lock) {
            long end = SystemClock.uptimeMillis() + 2000;
            while ((which == 2 ? bgHandler2 : bgHandler3) == null) {
                long left = end - SystemClock.uptimeMillis();
                if (left <= 0) {
                    break;
                }
                try {
                    lock.wait(left);
                } catch (InterruptedException e) {
                }
            }
        }
        check("t" + which + "-ready", (which == 2 ? bgHandler2 : bgHandler3) != null);
    }

    private void stage2() {
        t1 = new Thread(new Runnable() {
            public void run() {
                bgIsNotMain = Thread.currentThread() != mainThread;
                bgNoLooper = Looper.myLooper() == null;
                bgSawMainLooper = Looper.getMainLooper() != null;
                try {
                    new Handler();
                } catch (RuntimeException e) {
                    noLooperHandlerThrows = true;
                }
                try {
                    tv.setText("from-background-thread");
                } catch (Throwable e) {
                    wrongThreadThrown = e.getClass().getName().indexOf("CalledFromWrongThreadException") >= 0;
                }
                try {
                    Thread.sleep(500);
                } catch (InterruptedException e) {
                }
                mainHandler.post(r("bg500"));
            }
        }, "t1-worker");
        check("t1-name", "t1-worker".equals(t1.getName()));
        check("t1-new-state", t1.getState() == Thread.State.NEW);
        check("t1-not-alive-before-start", !t1.isAlive());
        t1.start();
        check("t1-alive-after-start", t1.isAlive());
        long s = SystemClock.uptimeMillis();
        try {
            Thread.sleep(25);
        } catch (InterruptedException e) {
        }
        check("sleep-duration", SystemClock.uptimeMillis() - s >= 20);

        t2 = new Thread(new Runnable() {
            public void run() {
                Looper.prepare();
                final Looper me = Looper.myLooper();
                try {
                    Looper.prepare();
                } catch (RuntimeException e) {
                    dupPrepareThrows = true;
                }
                Handler h = new Handler() {
                    public void handleMessage(Message m) {
                        if (Looper.myLooper() != me || Thread.currentThread() == mainThread) {
                            t2CallbacksOnT2 = false;
                        }
                        if (m.what == 7) {
                            mainHandler.post(r("echo7"));
                        } else if (m.what == 8) {
                            mainHandler.post(new Runnable() {
                                public void run() {
                                    step("echo8");
                                    bgHandler2.getLooper().quit();
                                }
                            });
                        }
                    }
                };
                synchronized (lock) {
                    bgHandler2 = h;
                    lock.notifyAll();
                }
                Looper.loop();
            }
        }, "t2-looper");
        t2.start();
        waitReady(2);
        bgHandler2.sendEmptyMessage(7);
        bgHandler2.post(new Runnable() {
            public void run() {
                if (Thread.currentThread() == mainThread) {
                    t2CallbacksOnT2 = false;
                }
                mainHandler.post(r("echoRun"));
            }
        });
        bgHandler2.sendEmptyMessageDelayed(8, 100);

        t3 = new Thread(new Runnable() {
            public void run() {
                Looper.prepare();
                Handler h = new Handler() {
                    public void handleMessage(Message m) {
                        t3Late = true;
                    }
                };
                synchronized (lock) {
                    bgHandler3 = h;
                    lock.notifyAll();
                }
                Looper.loop();
            }
        }, "t3-looper");
        t3.start();
        waitReady(3);
        bgHandler3.sendEmptyMessageDelayed(1, 4000);
        bgHandler3.getLooper().quitSafely();
        check("send-after-quit-rejected", !bgHandler3.sendEmptyMessage(2));
    }

    private void conclude() {
        Thread[] all = new Thread[] {t1, t2, t3};
        for (int i = 0; i < all.length; i++) {
            try {
                all[i].join(2000);
            } catch (InterruptedException e) {
            }
            check("thread-" + (i + 1) + "-terminated", !all[i].isAlive() && all[i].getState() == Thread.State.TERMINATED);
        }
        check("callback-order", mainLog.toString().equals(EXPECTED));
        check("all-callbacks-on-main", allOnMain);
        check("d300-delay", d300At - t0 >= 290);
        check("bg-not-main-thread", bgIsNotMain);
        check("bg-has-no-looper", bgNoLooper);
        check("bg-sees-main-looper", bgSawMainLooper);
        check("bg-handler-needs-looper", noLooperHandlerThrows);
        check("bg-ui-touch-rejected", wrongThreadThrown);
        check("dup-prepare-rejected", dupPrepareThrows);
        check("t2-callbacks-on-t2", t2CallbacksOnT2);
        check("t3-quitSafely-dropped-future", !t3Late);
        boolean quitRejected = false;
        try {
            Looper.getMainLooper().quit();
        } catch (IllegalStateException e) {
            quitRejected = true;
        }
        check("main-quit-rejected", quitRejected);
        finishWith(null);
    }

    private void finishWith(String why) {
        if (done) {
            return;
        }
        done = true;
        if (why != null) {
            failures.append(' ').append(why);
        }
        if (failures.length() == 0) {
            tv.setText("THREADTEST ALL PASS");
        } else {
            tv.setText("THREADTEST FAILED:" + failures + " log=" + mainLog);
        }
    }
}
