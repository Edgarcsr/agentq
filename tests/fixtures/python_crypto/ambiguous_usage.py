from cryptography.hazmat.primitives.asymmetric import rsa

key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
key = load_key()
key.sign(data, padding, algorithm)
monkey_key.sign(data)