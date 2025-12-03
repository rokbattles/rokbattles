package com.sylketech.rokbattles.mail;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import tools.jackson.databind.JsonNode;
import tools.jackson.databind.json.JsonMapper;
import tools.jackson.databind.node.ArrayNode;
import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import org.junit.jupiter.api.Test;

class MailDecoderNumericNormalizationTest {
    private final MailDecoder decoder = new MailDecoder();
    private static final JsonMapper MAPPER = JsonMapper.builder().build();

    @Test
    void normalizesWholeFloatsToIntegers() throws Exception {
        byte[] payload = singleSectionBuffer();
        ArrayNode sections = decoder.decodeSections(payload);
        JsonNode section = sections.get(0);

        assertEquals(MAPPER.readTree("""
                {
                  "int_from_f32": 7,
                  "long_from_f64": 12345678901,
                  "float_value": 1.5,
                  "nan_value": null
                }
                """), section);
    }

    private static byte[] singleSectionBuffer() throws Exception {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        writeStringKey(out, "int_from_f32");
        out.write(0x02);
        out.write(floatToBytes(7.0f));

        writeStringKey(out, "long_from_f64");
        out.write(0x03);
        out.write(doubleToBytes(12_345_678_901d));

        writeStringKey(out, "float_value");
        out.write(0x02);
        out.write(floatToBytes(1.5f));

        writeStringKey(out, "nan_value");
        out.write(0x03);
        out.write(doubleToBytes(Double.NaN));

        out.write(0xFF); // section terminator
        byte[] buffer = out.toByteArray();
        assertTrue(buffer.length > 0, "fixture should not be empty");
        return buffer;
    }

    private static void writeStringKey(ByteArrayOutputStream out, String key) throws Exception {
        byte[] bytes = key.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        out.write(0x04);
        out.write(intToLE(bytes.length));
        out.write(bytes);
    }

    private static byte[] intToLE(int value) {
        return ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(value).array();
    }

    private static byte[] floatToBytes(float value) {
        return ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putFloat(value).array();
    }

    private static byte[] doubleToBytes(double value) {
        // big-endian per source format
        return ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN).putDouble(value).array();
    }
}
