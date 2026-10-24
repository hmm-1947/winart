package org.xmlpull.v1;
import java.io.IOException;
public interface XmlPullParser {
    String FEATURE_PROCESS_NAMESPACES = "http://xmlpull.org/v1/doc/features.html#process-namespaces";
    int START_DOCUMENT = 0;
    int END_DOCUMENT = 1;
    int START_TAG = 2;
    int END_TAG = 3;
    int TEXT = 4;
    int CDSECT = 5;
    int ENTITY_REF = 6;
    int IGNORABLE_WHITESPACE = 7;
    int PROCESSING_INSTRUCTION = 8;
    int COMMENT = 9;
    int DOCDECL = 10;
    void setFeature(String name, boolean state) throws XmlPullParserException;
    int nextToken() throws XmlPullParserException, IOException;
    void require(int type, String namespace, String name) throws XmlPullParserException, IOException;
}
