package com.android.org.kxml2.io;
import java.io.IOException;
import java.io.InputStream;
import java.io.Reader;
import org.xmlpull.v1.XmlPullParser;
import org.xmlpull.v1.XmlPullParserException;
public class KXmlParser implements XmlPullParser, java.io.Closeable {
    public void keepNamespaceAttributes() {}
    public String getRootElementName() { return null; }
    public String getPublicId() { return null; }
    public String getSystemId() { return null; }
    public void close() throws IOException {}
    public void setFeature(String name, boolean state) throws XmlPullParserException {}
    public void setInput(Reader r) throws XmlPullParserException {}
    public void setInput(InputStream is, String enc) throws XmlPullParserException {}
    public int nextToken() throws XmlPullParserException, IOException { return END_DOCUMENT; }
    public void require(int type, String namespace, String name) throws XmlPullParserException, IOException {}
    public String getName() { return null; }
    public String getNamespace() { return null; }
    public String getText() { return null; }
    public int getAttributeCount() { return 0; }
    public String getAttributeName(int i) { return null; }
    public String getAttributeNamespace(int i) { return null; }
    public String getAttributePrefix(int i) { return null; }
    public String getAttributeValue(int i) { return null; }
    public String getPrefix() { return null; }
    public String getInputEncoding() { return null; }
    public int getDepth() { return 0; }
    public boolean getFeature(String name) { return false; }
    public Object getProperty(String name) { return null; }
    public String getNamespace(String prefix) { return null; }
    public int getEventType() { return 0; }
}
