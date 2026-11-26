import static java.security.MessageDigest.getInstance;

class Imports {
    static final String ALGORITHM = "SHA-" + 256;

    void inspect() throws Exception {
        getInstance(ALGORITHM); // @expect sha2-256:hash
        java.security.MessageDigest.getInstance("MD5", "SUN"); // @expect md5:hash
        javax.crypto.Cipher.getInstance("RSA"); // @expect rsa:encryption
    }
}
