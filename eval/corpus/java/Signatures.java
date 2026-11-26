import java.security.Signature;

class Signatures {
    void inspect() throws Exception {
        Signature.getInstance("SHA256withRSA"); // @expect rsa:signature
        Signature.getInstance("RSASSA-PSS"); // @expect rsa:signature
        Signature.getInstance("SHA1withDSA"); // @expect dsa:signature
        Signature.getInstance("SHA384withECDSAinP1363Format"); // @expect ecdsa:signature
        Signature.getInstance("Ed25519"); // @expect ed25519:signature
        Signature.getInstance("ML-DSA"); // @expect ml-dsa:signature
        Signature.getInstance("ML-DSA-65"); // @expect ml-dsa:signature
        Signature.getInstance("SLH-DSA-SHA2-128s", "BC"); // @expect slh-dsa:signature
    }
}
