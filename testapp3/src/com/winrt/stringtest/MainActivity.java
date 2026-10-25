package com.winrt.stringtest;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.StringTokenizer;

public class MainActivity extends Activity {
    private TextView tv;
    private int total;
    private int failed;

    static class Thing {
        private final int id;

        Thing(int id) {
            this.id = id;
        }

        @Override
        public String toString() {
            return "thing#" + id;
        }
    }

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

    private void eqInt(String name, int expected, int actual) {
        total++;
        if (expected != actual) {
            fail(name, Integer.toString(expected), Integer.toString(actual));
        }
    }

    private void ok(String name, boolean cond) {
        total++;
        if (!cond) {
            fail(name, "true", "false");
        }
    }

    private void boom(String name, Throwable t) {
        total++;
        failed++;
        tv.setText("EXCEPTION");
        tv.setText(name);
        try {
            tv.setText(t.toString());
        } catch (Throwable ignored) {
            tv.setText("<toString failed>");
        }
    }

    private void runAll() {
        try { sbAppend(); } catch (Throwable t) { boom("sbAppend", t); }
        try { sbEdit(); } catch (Throwable t) { boom("sbEdit", t); }
        try { sbGrow(); } catch (Throwable t) { boom("sbGrow", t); }
        try { sbUnicode(); } catch (Throwable t) { boom("sbUnicode", t); }
        try { concat(); } catch (Throwable t) { boom("concat", t); }
        try { valueOf(); } catch (Throwable t) { boom("valueOf", t); }
        try { numbers(); } catch (Throwable t) { boom("numbers", t); }
        try { strBasics(); } catch (Throwable t) { boom("strBasics", t); }
        try { strTransform(); } catch (Throwable t) { boom("strTransform", t); }
        try { strCase(); } catch (Throwable t) { boom("strCase", t); }
        try { strCompare(); } catch (Throwable t) { boom("strCompare", t); }
        try { strArrays(); } catch (Throwable t) { boom("strArrays", t); }
        try { strSplit(); } catch (Throwable t) { boom("strSplit", t); }
        try { strBytes(); } catch (Throwable t) { boom("strBytes", t); }
        try { strBounds(); } catch (Throwable t) { boom("strBounds", t); }
        try { strIdentity(); } catch (Throwable t) { boom("strIdentity", t); }
        try { collections(); } catch (Throwable t) { boom("collections", t); }
        try { switching(); } catch (Throwable t) { boom("switching", t); }
        try { resources(); } catch (Throwable t) { boom("resources", t); }
        try { nativeBridge(); } catch (Throwable t) { boom("nativeBridge", t); }
        try { exceptions(); } catch (Throwable t) { boom("exceptions", t); }
        try { classNames(); } catch (Throwable t) { boom("classNames", t); }
        try { regex(); } catch (Throwable t) { boom("regex", t); }
        try { floating(); } catch (Throwable t) { boom("floating", t); }
        try { format(); } catch (Throwable t) { boom("format", t); }
        tv.setText(Integer.toString(total));
        tv.setText(failed == 0 ? "STRINGTEST ALL PASS" : "STRINGTEST FAILED");
    }

    private void sbAppend() {
        StringBuilder sb = new StringBuilder();
        sb.append("abc").append(1).append('x').append(true).append(2L).append((Object) "o");
        eq("sb.append.mixed", "abc1xtrue2o", sb.toString());
        eqInt("sb.length", 11, sb.length());
        eq("sb.append.nullString", "null", new StringBuilder().append((String) null).toString());
        eq("sb.append.nullObject", "null", new StringBuilder().append((Object) null).toString());
        eq("sb.append.charArray", "hi", new StringBuilder().append(new char[] {'h', 'i'}).toString());
        eq("sb.append.charArrayRange", "bc", new StringBuilder().append(new char[] {'a', 'b', 'c'}, 1, 2).toString());
        eq("sb.append.csRange", "el", new StringBuilder().append("hello", 1, 3).toString());
        eq("sb.append.stringBuilder", "ab-cd", new StringBuilder("ab-").append(new StringBuilder("cd")).toString());
        eq("sb.append.negInt", "-42", new StringBuilder().append(-42).toString());
        eq("sb.append.intMin", "-2147483648", new StringBuilder().append(Integer.MIN_VALUE).toString());
        eq("sb.append.longMax", "9223372036854775807", new StringBuilder().append(Long.MAX_VALUE).toString());
        eq("sb.append.object", "thing#7", new StringBuilder().append(new Thing(7)).toString());
        eq("sb.empty", "", new StringBuilder().toString());
        eq("sb.ctor.string", "init", new StringBuilder("init").toString());
        eq("sb.ctor.capacity", "", new StringBuilder(64).toString());
        eq("sb.ctor.charSeq", "cs", new StringBuilder((CharSequence) "cs").toString());
        StringBuilder snap = new StringBuilder("a");
        String s = snap.toString();
        snap.append("b");
        eq("sb.toString.copy", "a", s);
        eq("sb.toString.after", "ab", snap.toString());
        ok("sb.capacity", new StringBuilder("abc").capacity() >= 3);
    }

    private void sbEdit() {
        eq("sb.insert.string", "Xhello", new StringBuilder("hello").insert(0, "X").toString());
        eq("sb.insert.int", "he42llo", new StringBuilder("hello").insert(2, 42).toString());
        eq("sb.insert.charEnd", "hi!", new StringBuilder("hi").insert(2, '!').toString());
        eq("sb.reverse", "cba", new StringBuilder("abc").reverse().toString());
        eq("sb.deleteCharAt", "ac", new StringBuilder("abc").deleteCharAt(1).toString());
        eq("sb.delete", "aef", new StringBuilder("abcdef").delete(1, 4).toString());
        eq("sb.replace", "aZZZdef", new StringBuilder("abcdef").replace(1, 3, "ZZZ").toString());
        StringBuilder set = new StringBuilder("abc");
        set.setCharAt(1, 'X');
        eq("sb.setCharAt", "aXc", set.toString());
        StringBuilder shrink = new StringBuilder("abcdef");
        shrink.setLength(2);
        eq("sb.setLength.shrink", "ab", shrink.toString());
        StringBuilder grow = new StringBuilder("ab");
        grow.setLength(3);
        eqInt("sb.setLength.grow.len", 3, grow.length());
        eqInt("sb.setLength.grow.nul", 0, grow.charAt(2));
        StringBuilder zero = new StringBuilder("abc");
        zero.setLength(0);
        eq("sb.setLength.zero", "", zero.toString());
        eqInt("sb.indexOf", 2, new StringBuilder("abcabc").indexOf("c"));
        eqInt("sb.lastIndexOf", 5, new StringBuilder("abcabc").lastIndexOf("c"));
        eqInt("sb.indexOf.from", 5, new StringBuilder("abcabc").indexOf("c", 3));
        eqInt("sb.charAt", 'b', new StringBuilder("abc").charAt(1));
        eq("sb.substring.1", "cdef", new StringBuilder("abcdef").substring(2));
        eq("sb.substring.2", "bc", new StringBuilder("abcdef").substring(1, 3));
        eq("sb.appendCodePoint", "A", new StringBuilder().appendCodePoint(65).toString());
        eqInt("sb.appendCodePoint.supp", 2, new StringBuilder().appendCodePoint(0x1F600).length());
    }

    private void sbGrow() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 200; i++) {
            sb.append("ab");
        }
        eqInt("sb.grow.len", 400, sb.length());
        eqInt("sb.grow.last", 'b', sb.charAt(399));
        eqInt("sb.grow.str.len", 400, sb.toString().length());
        StringBuilder digits = new StringBuilder();
        for (int i = 0; i < 10; i++) {
            digits.append(i);
        }
        eq("sb.loop.digits", "0123456789", digits.toString());
        StringBuilder big = new StringBuilder(2);
        for (int i = 0; i < 1000; i++) {
            big.append((char) ('a' + (i % 26)));
        }
        eqInt("sb.small.cap.grow", 1000, big.length());
        eqInt("sb.small.cap.char", 'b', big.charAt(27));
    }

    private void sbUnicode() {
        String cn = new StringBuilder("a").append('\u4e2d').append("b").toString();
        eq("sb.utf16.value", "a\u4e2db", cn);
        eqInt("sb.utf16.len", 3, cn.length());
        eqInt("sb.utf16.charAt", 0x4e2d, cn.charAt(1));
        eq("sb.latin1.accent", "caf\u00e9", new StringBuilder("caf").append('\u00e9').toString());
        eqInt("lit.utf16.len", 2, "\u4e2d\u6587".length());
        eqInt("lit.utf16.charAt", 0x6587, "\u4e2d\u6587".charAt(1));
        eqInt("lit.utf16.hash", 0x4e2d, "\u4e2d".hashCode());
        eq("sb.utf16.reverse", "b\u4e2da", new StringBuilder("a\u4e2db").reverse().toString());
        eq("sb.utf16.mixAppend", "\u00e9\u4e2dx", new StringBuilder("\u00e9").append("\u4e2d").append('x').toString());
    }

    private void concat() {
        String a = "foo";
        int n = 5;
        long l = 7L;
        char c = 'z';
        boolean b = true;
        Object nul = null;
        String nullStr = null;
        eq("concat.mixed", "foo5z7truenull", a + n + c + l + b + nul);
        int x = 1;
        int y = 2;
        eq("concat.intFirst", "3z", x + y + "z");
        eq("concat.strFirst", "z12", "z" + x + y);
        eq("concat.objects", "thing#3/thing#4", new Thing(3) + "/" + new Thing(4));
        String s = "";
        for (int i = 0; i < 5; i++) {
            s += i;
        }
        eq("concat.loop", "01234", s);
        String s2 = "x";
        s2 += "y";
        s2 += 'z';
        eq("concat.pluseq", "xyz", s2);
        char base = 'a';
        eq("concat.charArith", "b", "" + (char) (base + 1));
        eq("concat.method", "ab", "a".concat("b"));
        eq("concat.method.empty", "x", "x".concat(""));
        eq("concat.nullVar", "nullx", nullStr + "x");
        long big = Long.MAX_VALUE;
        eq("concat.long", "x9223372036854775807", "x" + big);
        String dyn = String.valueOf(12) + String.valueOf('c');
        eq("concat.dynamic", "12c!", dyn + "!");
        String r1 = new StringBuilder("ab").append("cd").toString();
        String r2 = String.valueOf(99);
        eq("concat.runtimeBoth", "abcd99", r1 + r2);
        eqInt("concat.len", 6, (r1 + r2).length());
    }

    private void valueOf() {
        eq("valueOf.int", "42", String.valueOf(42));
        eq("valueOf.long", "-7", String.valueOf(-7L));
        eq("valueOf.char", "c", String.valueOf('c'));
        eq("valueOf.true", "true", String.valueOf(true));
        eq("valueOf.false", "false", String.valueOf(false));
        eq("valueOf.nullObject", "null", String.valueOf((Object) null));
        eq("valueOf.object", "thing#9", String.valueOf(new Thing(9)));
        eq("valueOf.string", "s", String.valueOf((Object) "s"));
        eq("valueOf.charArray", "ok", String.valueOf(new char[] {'o', 'k'}));
        eq("valueOf.charArrayRange", "bc", String.valueOf(new char[] {'a', 'b', 'c', 'd'}, 1, 2));
        eq("copyValueOf", "xy", String.copyValueOf(new char[] {'x', 'y'}));
        eq("Character.toString", "q", Character.toString('q'));
        eq("Integer.toString.min", "-2147483648", Integer.toString(Integer.MIN_VALUE));
        eq("Long.toString.big", "1234567890123", Long.toString(1234567890123L));
    }

    private void numbers() {
        eq("Integer.toString.radix", "ff", Integer.toString(255, 16));
        eq("Integer.toHexString", "ff", Integer.toHexString(255));
        eq("Integer.toBinaryString", "101", Integer.toBinaryString(5));
        eq("Integer.toOctalString", "10", Integer.toOctalString(8));
        eqInt("Integer.parseInt", 123, Integer.parseInt("123"));
        eqInt("Integer.parseInt.neg", -5, Integer.parseInt("-5"));
        eqInt("Integer.valueOf", 7, Integer.valueOf("7").intValue());
        eqInt("Integer.parseInt.runtime", 321, Integer.parseInt(new StringBuilder("32").append(1).toString()));
        ok("Long.parseLong", Long.parseLong("9000000000") == 9000000000L);
        ok("Boolean.parseBoolean", Boolean.parseBoolean("TRUE"));
        boolean caught = false;
        try {
            Integer.parseInt("abc");
        } catch (NumberFormatException e) {
            caught = true;
            eq("NumberFormatException.message", "For input string: \"abc\"", e.getMessage());
        }
        ok("NumberFormatException.thrown", caught);
        ok("Character.isDigit", Character.isDigit('7') && !Character.isDigit('x'));
        ok("Character.isLetter", Character.isLetter('x') && !Character.isLetter('7'));
        ok("Character.isWhitespace", Character.isWhitespace(' ') && !Character.isWhitespace('a'));
        eqInt("Character.toUpperCase", 'A', Character.toUpperCase('a'));
        eqInt("Character.getNumericValue", 7, Character.getNumericValue('7'));
    }

    private void strBasics() {
        String s = "Hello, World";
        eqInt("length", 12, s.length());
        eqInt("charAt", 'o', s.charAt(4));
        eq("substring.1", "World", s.substring(7));
        eq("substring.2", "Hello", s.substring(0, 5));
        eq("substring.empty", "", s.substring(3, 3));
        eq("substring.full", "Hello, World", s.substring(0));
        eqInt("indexOf.char", 4, s.indexOf('o'));
        eqInt("indexOf.char.from", 8, s.indexOf('o', 5));
        eqInt("indexOf.string", 7, s.indexOf("World"));
        eqInt("indexOf.missing", -1, s.indexOf("zz"));
        eqInt("indexOf.empty", 0, s.indexOf(""));
        eqInt("lastIndexOf.char", 8, s.lastIndexOf('o'));
        eqInt("lastIndexOf.string", 8, s.lastIndexOf("o"));
        eqInt("lastIndexOf.char.from", 4, s.lastIndexOf('o', 7));
        ok("contains", s.contains("lo, W"));
        ok("contains.missing", !s.contains("xyz"));
        ok("startsWith", s.startsWith("Hell"));
        ok("startsWith.offset", s.startsWith("World", 7));
        ok("endsWith", s.endsWith("rld"));
        ok("isEmpty.true", "".isEmpty());
        ok("isEmpty.false", !s.isEmpty());
        eqInt("codePointAt", 'H', s.codePointAt(0));
        String rt = new StringBuilder("Hello, ").append("World").toString();
        eqInt("runtime.length", 12, rt.length());
        eq("runtime.substring", "World", rt.substring(7));
        eqInt("runtime.indexOf", 7, rt.indexOf("World"));
        ok("runtime.contains", rt.contains("o, W"));
        ok("runtime.startsWith", rt.startsWith("Hello"));
        ok("runtime.endsWith", rt.endsWith("World"));
    }

    private void strTransform() {
        eq("trim", "pad", "  pad \t".trim());
        eq("trim.noop", "pad", "pad".trim());
        eq("trim.allSpace", "", "   ".trim());
        eq("replace.char", "HeLLo, WorLd", "Hello, World".replace('l', 'L'));
        eq("replace.cs", "Hello, There", "Hello, World".replace("World", "There"));
        eq("replace.cs.many", "a|b|c", "a, b, c".replace(", ", "|"));
        eq("replace.cs.empty", "abc", "abc".replace("zz", "y"));
        eq("replace.cs.rt", "x-y", new StringBuilder("x+y").toString().replace('+', '-'));
        eq("join.varargs", "a,b,c", String.join(",", "a", "b", "c"));
        List<String> list = new ArrayList<String>();
        list.add("p");
        list.add("q");
        eq("join.iterable", "p-q", String.join("-", list));
        eq("concat.chain", "abc", "a".concat("b").concat("c"));
        eq("toString.identity", "same", "same".toString());
        eq("subSequence", "ell", "Hello".subSequence(1, 4).toString());
        char[] rev = "abc".toCharArray();
        for (int i = 0, j = rev.length - 1; i < j; i++, j--) {
            char t = rev[i];
            rev[i] = rev[j];
            rev[j] = t;
        }
        eq("manual.reverse", "cba", new String(rev));
    }

    private void strCase() {
        eq("toUpperCase", "HELLO, WORLD", "Hello, World".toUpperCase());
        eq("toLowerCase", "hello, world", "Hello, World".toLowerCase());
        eq("toUpperCase.rt", "ABC", new StringBuilder("a").append("bc").toString().toUpperCase());
        eq("toLowerCase.rt", "abc", new StringBuilder("A").append("BC").toString().toLowerCase());
        eq("toUpperCase.accent", "CAF\u00c9", "caf\u00e9".toUpperCase());
        eq("toLowerCase.digits", "a1b2", "A1B2".toLowerCase());
        ok("equalsIgnoreCase", "HeLLo".equalsIgnoreCase("hello"));
        ok("equalsIgnoreCase.false", !"abc".equalsIgnoreCase("abd"));
        eqInt("compareToIgnoreCase", 0, "ABC".compareToIgnoreCase("abc"));
    }

    private void strCompare() {
        ok("equals.same", "abc".equals("abc"));
        ok("equals.null", !"abc".equals(null));
        ok("equals.otherType", !"abc".equals(new Thing(1)));
        ok("equals.runtime", new StringBuilder("ab").append("c").toString().equals("abc"));
        ok("equals.runtime.rev", "abc".equals(new StringBuilder("ab").append("c").toString()));
        ok("contentEquals", "abc".contentEquals(new StringBuilder("abc")));
        eqInt("compareTo.less", -1, "Hello, World".compareTo("Hello, Xorld"));
        eqInt("compareTo.greater", 1, "b".compareTo("a"));
        eqInt("compareTo.prefix", -1, "abc".compareTo("abcd"));
        eqInt("compareTo.equal", 0, "abc".compareTo("abc"));
        eqInt("compareTo.rt", 0, new StringBuilder("ab").append("c").toString().compareTo("abc"));
        eqInt("hashCode.abc", 96354, "abc".hashCode());
        eqInt("hashCode.hello", 99162322, "hello".hashCode());
        eqInt("hashCode.empty", 0, "".hashCode());
        String rt = new StringBuilder("he").append("llo").toString();
        eqInt("hashCode.runtime", 99162322, rt.hashCode());
        eqInt("hashCode.twice", rt.hashCode(), rt.hashCode());
        ok("regionMatches", "Hello".regionMatches(1, "xell", 1, 3));
        ok("regionMatches.ic", "Hello".regionMatches(true, 0, "HE", 0, 2));
    }

    private void strArrays() {
        char[] cs = "abc".toCharArray();
        eqInt("toCharArray.len", 3, cs.length);
        eqInt("toCharArray.1", 'b', cs[1]);
        eq("new.charArray", "ok", new String(new char[] {'o', 'k'}));
        eq("new.charArrayRange", "bc", new String(new char[] {'a', 'b', 'c', 'd'}, 1, 2));
        eq("new.string", "copy", new String("copy"));
        eq("new.empty", "", new String());
        eq("new.builder", "bld", new String(new StringBuilder("bld")));
        char[] dst = new char[4];
        "wxyz".getChars(1, 3, dst, 1);
        eqInt("getChars.0", 0, dst[0]);
        eqInt("getChars.1", 'x', dst[1]);
        eqInt("getChars.2", 'y', dst[2]);
        eq("chars.roundtrip", "round", new String("round".toCharArray()));
    }

    private void strSplit() {
        String[] p = "a,b,,c".split(",");
        eqInt("split.len", 4, p.length);
        eq("split.0", "a", p[0]);
        eq("split.2", "", p[2]);
        eq("split.3", "c", p[3]);
        eqInt("split.trailing", 3, "a,b,c,,".split(",").length);
        eqInt("split.limit", 5, "a,b,c,,".split(",", -1).length);
        eqInt("split.none", 1, "abc".split(",").length);
        eqInt("split.rt", 2, new StringBuilder("x").append(";y").toString().split(";").length);
        StringTokenizer st = new StringTokenizer("a b  c");
        eqInt("tokenizer.count", 3, st.countTokens());
        eq("tokenizer.first", "a", st.nextToken());
    }

    private void strBytes() {
        byte[] b = "abc".getBytes();
        eqInt("getBytes.len", 3, b.length);
        eqInt("getBytes.1", 'b', b[1]);
        eq("new.bytes", "abc", new String(b));
        eq("new.bytes.range", "bc", new String(b, 1, 2));
        byte[] u = "\u00e9\u4e2d".getBytes();
        eqInt("getBytes.utf8.len", 5, u.length);
        eq("new.bytes.utf8", "\u00e9\u4e2d", new String(u));
    }

    private void strBounds() {
        boolean a = false;
        try {
            "abc".charAt(5);
        } catch (StringIndexOutOfBoundsException e) {
            a = true;
        }
        ok("charAt.oob", a);
        boolean b = false;
        try {
            "abc".substring(2, 1);
        } catch (StringIndexOutOfBoundsException e) {
            b = true;
        }
        ok("substring.oob", b);
        boolean c = false;
        try {
            "abc".substring(4);
        } catch (IndexOutOfBoundsException e) {
            c = true;
        }
        ok("substring.oob.start", c);
        boolean d = false;
        try {
            new StringBuilder("ab").charAt(9);
        } catch (IndexOutOfBoundsException e) {
            d = true;
        }
        ok("sb.charAt.oob", d);
        boolean e2 = false;
        String nul = null;
        try {
            nul.length();
        } catch (NullPointerException e) {
            e2 = true;
        }
        ok("null.length.npe", e2);
    }

    private void strIdentity() {
        String lit = "abc";
        String rt = new StringBuilder("ab").append("c").toString();
        ok("identity.literals", lit == "abc");
        ok("identity.runtime.differs", rt != lit);
        ok("identity.intern", rt.intern() == lit);
        ok("identity.intern.self", lit.intern() == lit);
        String fresh = new StringBuilder("zz").append("top").toString();
        String i1 = fresh.intern();
        String i2 = new StringBuilder("zz").append("top").toString().intern();
        ok("identity.intern.twice", i1 == i2);
        ok("identity.new.string", new String("abc") != lit);
    }

    private void collections() {
        Map<String, Integer> m = new HashMap<String, Integer>();
        m.put("cat", 1);
        m.put(new StringBuilder("do").append("g").toString(), 2);
        String rtCat = new StringBuilder("c").append("at").toString();
        eqInt("map.get.runtimeKey", 1, m.get(rtCat));
        eqInt("map.get.literalKey", 2, m.get("dog"));
        ok("map.containsKey", m.containsKey(rtCat));
        ok("map.missing", !m.containsKey("bird"));
        eqInt("map.size", 2, m.size());
        HashSet<String> set = new HashSet<String>();
        set.add("a");
        set.add(new StringBuilder("a").toString());
        eqInt("set.dedupe", 1, set.size());
        List<String> list = new ArrayList<String>();
        list.add("x");
        list.add("y");
        ok("list.contains.runtime", list.contains(new StringBuilder("y").toString()));
        eqInt("list.indexOf.runtime", 1, list.indexOf(new StringBuilder("y").toString()));
        String[] arr = {"b", "c", "a"};
        Arrays.sort(arr);
        eq("sort.strings", "abc", arr[0] + arr[1] + arr[2]);
        eq("Arrays.toString", "[a, b]", Arrays.toString(new String[] {"a", "b"}));
        eq("list.toString", "[x, y]", list.toString());
        eq("map.toString", "{k=v}", singleMap().toString());
    }

    private Map<String, String> singleMap() {
        Map<String, String> m = new HashMap<String, String>();
        m.put("k", "v");
        return m;
    }

    private void switching() {
        String k = new StringBuilder("ca").append("t").toString();
        int r;
        switch (k) {
            case "dog":
                r = 1;
                break;
            case "cat":
                r = 2;
                break;
            default:
                r = 0;
        }
        eqInt("switch.runtimeString", 2, r);
        String lit = "dog";
        int q;
        switch (lit) {
            case "dog":
                q = 1;
                break;
            default:
                q = 0;
        }
        eqInt("switch.literal", 1, q);
    }

    private void resources() {
        String hello = getString(R.string.hello);
        eq("res.string", "Hello", hello);
        ok("res.equalsLiteral", hello.equals("Hello"));
        ok("res.literalEquals", "Hello".equals(hello));
        eqInt("res.length", 5, hello.length());
        eqInt("res.hash", "Hello".hashCode(), hello.hashCode());
        eq("res.concat", "Hello, world", hello + ", world");
        eq("res.sb", "[Hello]", new StringBuilder("[").append(hello).append(']').toString());
        eq("res.upper", "HELLO", hello.toUpperCase());
        eq("res.substring", "ell", hello.substring(1, 4));
        eqInt("res.indexOf", 1, hello.indexOf("ell"));
        eq("res.valueOf", "Hello", String.valueOf(hello));
        ok("res.intern", hello.intern() == "Hello");
        eq("res.format", "Hi Bob, you are 7", getString(R.string.greet_fmt, "Bob", 7));
        eq("res.format.concat", "Hi Bob, you are 7!", getString(R.string.greet_fmt, "Bob", 7) + "!");
        Map<String, Integer> m = new HashMap<String, Integer>();
        m.put("Hello", 5);
        eqInt("res.mapKey", 5, m.get(hello));
        int s;
        switch (hello) {
            case "Hello":
                s = 1;
                break;
            default:
                s = 0;
        }
        eqInt("res.switch", 1, s);
    }

    private void nativeBridge() {
        eq("pkg", "com.winrt.stringtest", getPackageName());
        eqInt("pkg.length", 20, getPackageName().length());
        eq("pkg.concat", "com.winrt.stringtest/x", getPackageName() + "/x");
        eq("pkg.sb", "[com.winrt.stringtest]", new StringBuilder("[").append(getPackageName()).append("]").toString());
        ok("pkg.startsWith", getPackageName().startsWith("com.winrt"));
        eqInt("pkg.lastIndexOf", 9, getPackageName().lastIndexOf('.'));
        tv.setText(new StringBuilder("RT").append(1).toString());
        eq("setText.roundtrip", "RT1", tv.getText().toString());
        tv.setText("lit" + 2);
        eq("setText.concat", "lit2", tv.getText().toString());
    }

    private void exceptions() {
        eq("exc.message.concat", "msg1", new RuntimeException("msg" + 1).getMessage());
        eq("exc.message.arg", "bad 3", new IllegalArgumentException("bad " + 3).getMessage());
        eq("exc.toString", "java.lang.RuntimeException: boom", new RuntimeException("boom").toString());
    }

    private void classNames() {
        eq("class.getName", "com.winrt.stringtest.MainActivity", getClass().getName());
        eq("class.getSimpleName", "MainActivity", getClass().getSimpleName());
        eq("class.string.name", "java.lang.String", "x".getClass().getName());
        eq("class.nested.name", "com.winrt.stringtest.MainActivity$Thing", new Thing(1).getClass().getName());
    }

    private void regex() {
        ok("regex.matches", "12345".matches("\\d+"));
        ok("regex.matches.false", !"12a45".matches("\\d+"));
        eq("regex.replaceAll", "h*ll*", "hello".replaceAll("[aeiou]", "*"));
        eq("regex.replaceFirst", "h*llo", "hello".replaceFirst("[aeiou]", "*"));
        eqInt("regex.split.ws", 3, "a  b \t c".split("\\s+").length);
    }

    private void floating() {
        eq("fp.double", "1.5", String.valueOf(1.5));
        eq("fp.double.whole", "100.0", String.valueOf(100.0));
        eq("fp.float", "2.5", String.valueOf(2.5f));
        eq("fp.sb.double", "3.75", new StringBuilder().append(3.75).toString());
        double d = 0.25;
        eq("fp.concat", "v=0.25", "v=" + d);
        eq("fp.Double.toString", "0.1", Double.toString(0.1));
        ok("fp.parse", Double.parseDouble("2.5") + 1 == 3.5);
    }

    private void format() {
        eq("format.d", "5-x", String.format("%d-%s", 5, "x"));
        eq("format.pad", "007", String.format("%03d", 7));
        eq("format.hex", "ff", String.format("%x", 255));
        eq("format.float", "3.14", String.format("%.2f", 3.14159));
    }
}
