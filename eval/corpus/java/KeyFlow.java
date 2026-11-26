import java.security.*;
import javax.crypto.*;

// Two RSA keys and two EC keys in one file: only the variables tell their
// purposes apart. The same local names are reused on purpose.
class KeyFlow {
    void sign() throws Exception {
        var generator = KeyPairGenerator.getInstance("RSA"); // @expect rsa:signature
        generator.initialize(3072);
        KeyPair pair = generator.generateKeyPair();
        var signer = Signature.getInstance("SHA256withRSA"); // @expect rsa:signature
        signer.initSign(pair.getPrivate());
    }

    void seal() throws Exception {
        var generator = KeyPairGenerator.getInstance("RSA"); // @expect rsa:encryption
        generator.initialize(2048);
        KeyPair pair = generator.generateKeyPair();
        PublicKey recipient = (PublicKey) pair.getPublic();
        var cipher = Cipher.getInstance("RSA/ECB/OAEPWithSHA-256AndMGF1Padding"); // @expect rsa:encryption
        cipher.init(Cipher.ENCRYPT_MODE, recipient);
    }

    void agree(String configured) throws Exception {
        var generator = KeyPairGenerator.getInstance("EC"); // @expect ecdh:key_exchange
        KeyPair pair = generator.generateKeyPair();
        var agreement = KeyAgreement.getInstance(configured);
        agreement.init(pair.getPrivate());
    }

    void verify(PublicKey peer) throws Exception {
        var generator = KeyPairGenerator.getInstance("EC"); // @expect ecdsa:signature
        var verifier = Signature.getInstance("SHA256withECDSA"); // @expect ecdsa:signature
        verifier.initSign(generator.generateKeyPair().getPrivate());
    }
}
