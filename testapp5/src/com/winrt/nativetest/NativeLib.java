package com.winrt.nativetest;

public class NativeLib {
    public int counter = 1;
    public static int total = 100;
    public String label = "init";
    public long big = 5L;
    public double ratio = 2.0;
    public float fl = 1.5f;
    public boolean flag = false;
    public byte by = 7;
    public char ch = 'a';
    public short sh = 10;

    static {
        System.loadLibrary("nativetest");
    }

    public static native int add(int a, int b);
    public static native long mulLong(long a, long b);
    public static native double hyp(double a, double b);
    public static native float scaleF(float a, float k);
    public static native boolean isEven(int n);
    public static native byte negByte(byte b);
    public static native char nextChar(char c);
    public static native short shortSum(short a, short b);
    public static native int many(int a, int b, int c, int d, int e, int f, int g, int h);
    public static native double manyD(double a, double b, double c, double d, double e, double f, double g, double h, double i, double j);
    public static native double mixed(int a, long b, double d, float f, byte g, char h);
    public static native String greet(String name, int n);
    public static native int[] strInfo(String s);
    public static native String regions(String s);
    public static native String fromChars(char[] a);
    public static native String viaCtor(char[] a);
    public static native int sumInts(int[] a);
    public static native void fillBytes(byte[] a, int v);
    public static native double[] makeDoubles(int n);
    public static native long sumLongs(long[] a);
    public static native int sortFirstLast(int[] a);
    public static native boolean[] flags(int n);
    public static native float sumFloats(float[] a);
    public static native void negShorts(short[] a);
    public static native String[] words(String csv);
    public static native int countNonNull(Object[] a);
    public static native int callTwice(int x);
    public static native String fmt(double d, long l, int i);
    public static native String libc(String s);
    public static native void thrower(String msg);
    public static native int catcher();
    public static native void propagate();
    public static native Object makePoint(int x, int y);
    public static native int[] virt(Base b);
    public static native int classes();
    public static native int tlsNext();
    public static native int initValue();
    public static native int registered(int x);
    public static native void keep(Object o);
    public static native Object kept();
    public static native boolean vmCheck();
    public static native boolean missingClass();
    public static native boolean sync(Object o);
    public static native void unbound();

    public native void bump();
    public native String callMix();
    public native String callMixA();
    public native String callMixV();

    public static int twice(int x) {
        return x * 2;
    }

    public static void boom() {
        throw new RuntimeException("boom");
    }

    public String describe(int i, long l, double d, float f, String s, boolean z, char c, byte b, short sh) {
        return i + "," + l + "," + d + "," + f + "," + s + "," + z + "," + c + "," + b + "," + sh;
    }
}
