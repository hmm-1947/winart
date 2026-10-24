package com.winrt.calc;

public class Calc {
    private long acc = 0;
    private long cur = 0;
    private char op = 0;
    private boolean fresh = true;

    public String press(String key) {
        char c = key.charAt(0);
        if (c >= '0' && c <= '9') {
            cur = fresh ? (c - '0') : cur * 10 + (c - '0');
            fresh = false;
            return Long.toString(cur);
        }
        if (c == 'C') {
            acc = 0;
            cur = 0;
            op = 0;
            fresh = true;
            return "0";
        }
        apply();
        if (c == '=') {
            op = 0;
        } else {
            op = c;
        }
        fresh = true;
        cur = acc;
        return Long.toString(acc);
    }

    private void apply() {
        if (op == 0) {
            acc = cur;
        } else if (op == '+') {
            acc = acc + cur;
        } else if (op == '-') {
            acc = acc - cur;
        } else if (op == '*') {
            acc = acc * cur;
        } else if (op == '/') {
            acc = cur == 0 ? 0 : acc / cur;
        }
    }
}
