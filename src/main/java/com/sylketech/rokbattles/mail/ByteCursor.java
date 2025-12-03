package com.sylketech.rokbattles.mail;

import java.nio.charset.StandardCharsets;

final class ByteCursor {
    private final byte[] buffer;
    private int offset;

    ByteCursor(byte[] buffer) {
        this.buffer = buffer;
        this.offset = 0;
    }

    boolean eof() {
        return offset >= buffer.length;
    }

    int position() {
        return offset;
    }

    int remaining() {
        return buffer.length - offset;
    }

    void moveTo(int newOffset) {
        this.offset = Math.max(0, Math.min(newOffset, buffer.length));
    }

    int u8() throws MailDecodeException {
        if (remaining() < 1) {
            throw new MailDecodeException("u8 past EOF");
        }
        return buffer[offset++] & 0xFF;
    }

    long u32Le() throws MailDecodeException {
        if (remaining() < 4) {
            throw new MailDecodeException("u32_le past EOF");
        }
        long value = readU32LE(buffer, offset);
        offset += 4;
        return value;
    }

    float f32Le() throws MailDecodeException {
        return Float.intBitsToFloat((int) u32Le());
    }

    double f64Be() throws MailDecodeException {
        if (remaining() < 8) {
            throw new MailDecodeException("f64_be past EOF");
        }
        long bits = ((long) buffer[offset] & 0xFF) << 56
                | ((long) buffer[offset + 1] & 0xFF) << 48
                | ((long) buffer[offset + 2] & 0xFF) << 40
                | ((long) buffer[offset + 3] & 0xFF) << 32
                | ((long) buffer[offset + 4] & 0xFF) << 24
                | ((long) buffer[offset + 5] & 0xFF) << 16
                | ((long) buffer[offset + 6] & 0xFF) << 8
                | ((long) buffer[offset + 7] & 0xFF);
        offset += 8;
        return Double.longBitsToDouble(bits);
    }

    String strUtf8() throws MailDecodeException {
        int len = (int) u32Le();
        if (len < 0 || remaining() < len) {
            throw new MailDecodeException("str_utf8 past EOF");
        }
        String value = new String(buffer, offset, len, StandardCharsets.UTF_8);
        offset += len;
        return value;
    }

    byte[] rawBuffer() {
        return buffer;
    }

    private static long readU32LE(byte[] buf, int index) {
        return ((long) buf[index] & 0xFF)
                | (((long) buf[index + 1] & 0xFF) << 8)
                | (((long) buf[index + 2] & 0xFF) << 16)
                | (((long) buf[index + 3] & 0xFF) << 24);
    }
}
