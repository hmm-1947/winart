package org.xmlpull.v1;
public class XmlPullParserException extends Exception {
    public Throwable getDetail() { return getCause(); }
    public int getLineNumber() { return -1; }
    public int getColumnNumber() { return -1; }
    public XmlPullParserException(String s) { super(s); }
}
