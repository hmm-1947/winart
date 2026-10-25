package com.winrt.nativetest;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

public class MainActivity extends Activity {
    private TextView tv;
    private int total;
    private int failed;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        tv = new TextView(this);
        setContentView(tv);
        runAll();
    }

    private void fail(String name, String expected, String actual) {
        failed++;
        tv.setText("FAIL");
        tv.setText(name);
        tv.setText(expected);
        tv.setText(actual == null ? "<null>" : actual);
    }

    private void eq(String name, String expected, String actual) {
        total++;
        boolean good = expected == null ? actual == null : expected.equals(actual);
        if (!good) {
            fail(name, expected, actual);
        }
    }

    private void eqInt(String name, long expected, long actual) {
        total++;
        if (expected != actual) {
            fail(name, Long.toString(expected), Long.toString(actual));
        }
    }

    private void eqDouble(String name, double expected, double actual) {
        total++;
        if (Math.abs(expected - actual) > 1e-9) {
            fail(name, Double.toString(expected), Double.toString(actual));
        }
    }

    private void ok(String name, boolean cond) {
        total++;
        if (!cond) {
            fail(name, "true", "false");
        }
    }

    private void runAll() {
        eqInt("add", 7, NativeLib.add(3, 4));
        eqInt("mulLong", 42L, NativeLib.mulLong(6L, 7L));
        eqDouble("hyp", 5.0, NativeLib.hyp(3.0, 4.0));
        eqDouble("scaleF", 7.5, NativeLib.scaleF(2.5f, 3.0f));
        ok("isEven true", NativeLib.isEven(4));
        ok("isEven false", !NativeLib.isEven(5));
        eqInt("negByte", -5, NativeLib.negByte((byte) 5));
        eqInt("nextChar", 'b', NativeLib.nextChar('a'));
        eqInt("shortSum", 30, NativeLib.shortSum((short) 10, (short) 20));
        eqInt("many", 1 + 2 * 2 + 3 * 3 + 4 * 4 + 5 * 5 + 6 * 6 + 7 * 7 + 8 * 8, NativeLib.many(1, 2, 3, 4, 5, 6, 7, 8));
        eqDouble("manyD", 1 + 2 * 2 + 3 * 3 + 4 * 4 + 5 * 5 + 6 * 6 + 7 * 7 + 8 * 8 + 9 * 9 + 10 * 10,
                NativeLib.manyD(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));
        eqDouble("mixed", 1 + 2 + 3.0 + 4.0f + 5 + 'a', NativeLib.mixed(1, 2L, 3.0, 4.0f, (byte) 5, 'a'));
        eq("greet", "Hello, World! n=42 hex=2a", NativeLib.greet("World", 42));

        int[] info = NativeLib.strInfo("abc");
        eqInt("strInfo len", 3, info[0]);
        eqInt("strInfo utflen", 3, info[1]);
        eqInt("strInfo firstchar", 'a', info[2]);

        eq("regions", "bcd", NativeLib.regions("abcde"));
        eq("fromChars", "xyz", NativeLib.fromChars(new char[] {'x', 'y', 'z'}));
        eq("viaCtor", "xyz", NativeLib.viaCtor(new char[] {'x', 'y', 'z'}));
        eqInt("sumInts", 15, NativeLib.sumInts(new int[] {1, 2, 3, 4, 5}));

        byte[] bytes = new byte[4];
        NativeLib.fillBytes(bytes, 10);
        eqInt("fillBytes0", 10, bytes[0]);
        eqInt("fillBytes3", 13, bytes[3]);

        double[] dbls = NativeLib.makeDoubles(4);
        eqDouble("makeDoubles2", 1.0, dbls[2]);
        eqInt("sumLongs", 60L, NativeLib.sumLongs(new long[] {10, 20, 30}));
        eqInt("sortFirstLast", 1005, NativeLib.sortFirstLast(new int[] {5, 3, 1, 4, 2}));

        boolean[] flags = NativeLib.flags(6);
        ok("flags0", flags[0]);
        ok("flags1", !flags[1]);
        ok("flags3", flags[3]);

        eqDouble("sumFloats", 6.0, NativeLib.sumFloats(new float[] {1.0f, 2.0f, 3.0f}));

        short[] shorts = {1, -2, 3};
        NativeLib.negShorts(shorts);
        eqInt("negShorts0", -1, shorts[0]);
        eqInt("negShorts1", 2, shorts[1]);

        String[] words = NativeLib.words("a,bb,ccc");
        eqInt("words len", 3, words.length);
        eq("words1", "bb", words[1]);

        eqInt("countNonNull", 2, NativeLib.countNonNull(new Object[] {"a", null, "b"}));
        eqInt("callTwice", 15, NativeLib.callTwice(7));
        eq("fmt", "2.500|100|00042|2a|str|q|  2.5|2.500000e+00", NativeLib.fmt(2.5, 100L, 42));
        eq("libc", "ABC:3:1234:255:1", NativeLib.libc("abc"));

        boolean threw = false;
        try {
            NativeLib.thrower("boom-msg");
        } catch (IllegalStateException e) {
            threw = true;
            eq("thrower msg", "boom-msg", e.getMessage());
        }
        ok("thrower threw", threw);

        eqInt("catcher", 1, NativeLib.catcher());

        boolean propagated = false;
        try {
            NativeLib.propagate();
        } catch (RuntimeException e) {
            propagated = true;
        }
        ok("propagate", propagated);

        Point p = (Point) NativeLib.makePoint(3, 4);
        eqInt("makePoint x", 3, p.x);
        eqInt("makePoint y", 4, p.y);

        Derived d = new Derived();
        int[] v = NativeLib.virt(d);
        eqInt("virt virtual", 2, v[0]);
        eqInt("virt nonvirtual", 1, v[1]);

        eqInt("classes", 7, NativeLib.classes());
        int t1 = NativeLib.tlsNext();
        int t2 = NativeLib.tlsNext();
        eqInt("tlsNext advances", 1, t2 - t1);
        eqInt("initValue", 42, NativeLib.initValue());
        eqInt("registered", 21, NativeLib.registered(7));

        String kept = "keepme";
        NativeLib.keep(kept);
        eq("kept", kept, (String) NativeLib.kept());

        ok("vmCheck", NativeLib.vmCheck());
        ok("missingClass", NativeLib.missingClass());
        ok("sync", NativeLib.sync(new Object()));

        NativeLib n = new NativeLib();
        n.bump();
        eqInt("bump counter", 2, n.counter);
        eqInt("bump total", 110, NativeLib.total);
        eq("bump label", "bumped", n.label);
        eqInt("bump big", 11L, n.big);
        eqDouble("bump ratio", 3.0, n.ratio);
        eqDouble("bump fl", 2.0, n.fl);
        ok("bump flag", n.flag);
        eqInt("bump by", 8, n.by);
        eqInt("bump ch", 'b', n.ch);
        eqInt("bump sh", 9, n.sh);

        eq("callMix", "7,1234567890123,2.5,1.25,str,true,x,-3,-300", n.callMix());
        eq("callMixA", "7,1234567890123,2.5,1.25,str,true,x,-3,-300", n.callMixA());
        eq("callMixV", "7,1234567890123,2.5,1.25,str,true,x,-3,-300", n.callMixV());

        tv.setText("checks: " + total + " failed: " + failed);
        if (failed == 0) {
            tv.setText("NATIVETEST ALL PASS");
        } else {
            tv.setText("NATIVETEST FAIL");
        }
    }
}
