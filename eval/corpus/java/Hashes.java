import java.security.MessageDigest;
import javax.crypto.Mac;
import javax.crypto.SecretKeyFactory;

class Hashes {
    void inspect() throws Exception {
        MessageDigest.getInstance("MD5"); // @expect md5:hash
        MessageDigest.getInstance("SHA-1"); // @expect sha1:hash
        MessageDigest.getInstance("SHA-256"); // @expect sha2-256:hash
        MessageDigest.getInstance("SHA-384"); // @expect sha2-384:hash
        MessageDigest.getInstance("SHA-512"); // @expect sha2-512:hash
        MessageDigest.getInstance("SHA3-256"); // @expect sha3:hash
        Mac.getInstance("HmacSHA256"); // @expect sha2-256:mac
        Mac.getInstance("HmacMD5"); // @expect md5:mac
        SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256"); // @expect sha2-256:key_derivation
        final String algorithm = "M" + "D5";
        MessageDigest.getInstance(algorithm); // @expect md5:hash
    }
}
