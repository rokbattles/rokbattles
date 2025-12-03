package com.sylketech.rokbattles.mail;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;

import tools.jackson.databind.JsonNode;
import tools.jackson.databind.json.JsonMapper;
import tools.jackson.databind.node.ArrayNode;
import java.io.IOException;
import java.io.InputStream;
import java.util.stream.Stream;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

class MailDecoderSamplesTest {
    private static final JsonMapper MAPPER = JsonMapper.builder().build();
    private final MailDecoder decoder = new MailDecoder();

    static Stream<Arguments> samples() {
        return Stream.of(
                Arguments.of("mail/Persistent.Mail.1002579517552941234", "mail/1002579517552941234.json"),
                Arguments.of("mail/Persistent.Mail.1036544617552967227", "mail/1036544617552967227.json"),
                Arguments.of("mail/Persistent.Mail.28700881175578047431", "mail/28700881175578047431.json"));
    }

    @ParameterizedTest
    @MethodSource("samples")
    void decodesSamples(String rawPath, String jsonPath) throws Exception {
        byte[] input = readBytes(rawPath);
        ArrayNode actual = decoder.decodeSections(input);
        JsonNode expected = readJson(jsonPath).path("sections");

        assertFalse(actual.isEmpty(), "decoded sections should not be empty");
        assertEquals(expected, actual, "decoded payload should match sample json");
    }

    private static byte[] readBytes(String path) throws IOException {
        try (InputStream in = openResource(path)) {
            return in.readAllBytes();
        }
    }

    private static JsonNode readJson(String path) throws IOException {
        try (InputStream in = openResource(path)) {
            return MAPPER.readTree(in);
        }
    }

    private static InputStream openResource(String path) {
        InputStream in = MailDecoderSamplesTest.class.getClassLoader().getResourceAsStream(path);
        assertNotNull(in, "resource not found: " + path);
        return in;
    }
}
