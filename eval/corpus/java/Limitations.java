import java.security.MessageDigest;
import java.security.Signature;

class Limitations {
    static String algorithm() { return "MD5"; }

    void indirect() throws Exception {
        MessageDigest.getInstance(algorithm()); // @expect md5:hash
        String signature = System.getenv().getOrDefault("SIGNATURE", "SHA256withRSA");
        Signature.getInstance(signature); // @expect rsa:signature
    }
}
