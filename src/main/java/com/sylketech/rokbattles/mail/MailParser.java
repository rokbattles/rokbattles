package com.sylketech.rokbattles.mail;

import tools.jackson.databind.JsonNode;
import tools.jackson.databind.node.ArrayNode;
import tools.jackson.databind.node.JsonNodeFactory;
import tools.jackson.databind.node.NullNode;
import tools.jackson.databind.node.ObjectNode;

final class MailParser {
    private static final int TAG_BOOLEAN = 0x01;
    private static final int TAG_F32 = 0x02;
    private static final int TAG_F64 = 0x03;
    private static final int TAG_STRING = 0x04;
    private static final int TAG_OBJECT = 0x05;
    private static final int TAG_SECTION_END = 0xFF;

    private final ByteCursor cursor;
    private final JsonNodeFactory json;

    MailParser(byte[] buffer, JsonNodeFactory json) {
        this.cursor = new ByteCursor(buffer);
        this.json = json;
    }

    ArrayNode parseSections() throws MailDecodeException {
        ArrayNode sections = json.arrayNode();
        while (!cursor.eof()) {
            if (!advanceToNextKey()) {
                break;
            }
            int before = cursor.position();
            ObjectNode obj = parseObject();
            if (!obj.isEmpty()) {
                sections.add(obj);
            }
            if (cursor.position() <= before) {
                // avoid infinite loops on malformed payloads
                break;
            }
        }
        return sections;
    }

    private ObjectNode parseObject() throws MailDecodeException {
        ObjectNode obj = json.objectNode();
        while (!cursor.eof()) {
            int tag = cursor.u8();
            if (tag == TAG_SECTION_END) {
                return obj;
            }
            if (tag != TAG_STRING) {
                continue;
            }
            String key = cursor.strUtf8();
            if (cursor.eof()) {
                obj.set(key, NullNode.getInstance());
                return obj;
            }
            int head = cursor.u8();
            JsonNode value = parseValue(head);
            obj.set(key, value);
        }
        return obj;
    }

    private JsonNode parseValue(int head) throws MailDecodeException {
        return switch (head) {
            case TAG_BOOLEAN -> json.booleanNode(cursor.u8() != 0);
            case TAG_F32 -> normalizeNumeric(cursor.f32Le());
            case TAG_F64 -> normalizeNumeric(cursor.f64Be());
            case TAG_STRING -> json.stringNode(cursor.strUtf8());
            case TAG_OBJECT -> parseObject();
            default -> NullNode.getInstance();
        };
    }

    private JsonNode normalizeNumeric(double value) {
        if (Double.isFinite(value) && Math.floor(value) == value) {
            if (value >= Integer.MIN_VALUE && value <= Integer.MAX_VALUE) {
                return json.numberNode((int) value);
            }
            if (value >= Long.MIN_VALUE && value <= Long.MAX_VALUE) {
                return json.numberNode((long) value);
            }
        }
        if (Double.isFinite(value)) {
            return json.numberNode(value);
        }
        return NullNode.getInstance();
    }

    private boolean advanceToNextKey() {
        byte[] buffer = cursor.rawBuffer();
        int off = cursor.position();
        int bufLen = buffer.length;

        while (off + 7 <= bufLen) {
            if ((buffer[off] & 0xFF) != TAG_STRING) {
                off++;
                continue;
            }
            if (off + 5 > bufLen) {
                return false;
            }
            long len = readU32LE(buffer, off + 1);
            if (len < 1 || len >= 1024) {
                off++;
                continue;
            }
            int start = off + 5;
            long endLong = start + len;
            if (endLong > bufLen) {
                off++;
                continue;
            }
            int end = (int) endLong;
            if (len >= 2 && isPrintableAscii(buffer, start, end)) {
                cursor.moveTo(off);
                return true;
            }
            off++;
        }
        cursor.moveTo(bufLen);
        return false;
    }

    private static boolean isPrintableAscii(byte[] buffer, int start, int end) {
        for (int i = start; i < end; i++) {
            int b = buffer[i] & 0xFF;
            if (b < 0x20 || b > 0x7E) {
                return false;
            }
        }
        return true;
    }

    private static long readU32LE(byte[] buffer, int index) {
        return ((long) buffer[index] & 0xFF)
                | (((long) buffer[index + 1] & 0xFF) << 8)
                | (((long) buffer[index + 2] & 0xFF) << 16)
                | (((long) buffer[index + 3] & 0xFF) << 24);
    }
}
