package android.util;

public interface AttributeSet {
    int getAttributeCount();

    String getAttributeName(int index);

    String getAttributeValue(int index);

    String getAttributeValue(String namespace, String name);

    int getAttributeNameResource(int index);

    int getAttributeResourceValue(String namespace, String name, int defaultValue);

    int getAttributeIntValue(String namespace, String name, int defaultValue);

    boolean getAttributeBooleanValue(String namespace, String name, boolean defaultValue);
}
