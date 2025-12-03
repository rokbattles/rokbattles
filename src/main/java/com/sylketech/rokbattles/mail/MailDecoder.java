package com.sylketech.rokbattles.mail;

import tools.jackson.databind.node.ArrayNode;
import tools.jackson.databind.node.JsonNodeFactory;

public final class MailDecoder {
    private final JsonNodeFactory json;

    public MailDecoder() {
        this(JsonNodeFactory.instance);
    }

    public MailDecoder(JsonNodeFactory json) {
        this.json = json;
    }

    public ArrayNode decodeSections(byte[] buffer) throws MailDecodeException {
        return new MailParser(buffer, json).parseSections();
    }
}
