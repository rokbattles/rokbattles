package com.sylketech.rokbattles.mail;

public class MailDecodeException extends Exception {
    public MailDecodeException(String message) {
        super(message);
    }

    public MailDecodeException(String message, Throwable cause) {
        super(message, cause);
    }
}
