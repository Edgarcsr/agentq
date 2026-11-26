# Analisador Java do agentq

O agentq é uma aplicação em Rust que analisa o código-fonte de outros projetos. A parte Java também foi implementada em Rust: sua função é entender arquivos `.java`, sem compilar nem executar o projeto analisado.

O projeto já tinha o scanner de arquivos, o parser `tree-sitter-java`, a interface `Analyzer`, a estrutura `Finding`, a inferência de finalidade, a base de conhecimento e os relatórios. O novo módulo conecta o Java a esse fluxo, usando as dependências existentes.

## Como o fluxo funciona

1. `src/detect.rs` encontra arquivos `.java`, respeitando as exclusões e o `.gitignore` já usados pelo projeto.
2. A CLI e a interface interativa chamam `analyze_directory(root, lang)`, em `src/analysis.rs`. Essa função compartilha a descoberta dos arquivos, a detecção e a avaliação. Dentro dela, `analyze_files` seleciona `JavaAnalyzer` para arquivos da linguagem Java.
3. `src/parse.rs` transforma o código em uma árvore sintática com a gramática Java existente.
4. O novo módulo identifica chamadas a APIs criptográficas conhecidas e produz registros `Finding` com arquivo, linha, coluna, trecho, biblioteca, algoritmo, finalidade, tamanho de chave e confiança.
5. `src/modules/java/keys.rs` segue cada chave pelas variáveis até a operação que a recebe (`initSign`, `initVerify`, `init`, `doPhase`), e `src/purpose.rs` usa essa ligação para resolver finalidades desconhecidas. Sem ligação, usa outras operações do mesmo arquivo como palpite fraco.
6. `src/assess.rs` consulta `src/knowledge/algorithms.toml` para atribuir risco, urgência e alternativas, recebendo também o idioma escolhido.
7. Os relatórios existentes apresentam os resultados em texto, JSON ou Markdown, incluindo os filtros `--hide-safe` e `--min-confidence`. A interface interativa usa os mesmos resultados na visão geral e na lista de achados.

A avaliação de risco e as recomendações continuam centralizadas na base de conhecimento existente. As regras Java traduzem nomes das APIs para os identificadores dessa base.

## Integração com a interface e os idiomas

O caminho da CLI é `main → analyze_directory → analyze_files → JavaAnalyzer.analyze`. Na interface interativa, `Tui` emite um `Cmd::Scan`; o runtime executa `analyze_directory` em uma thread e devolve um `Msg::Scanned`. O registro de `Language::Java` no seletor de analisadores atende aos dois caminhos.

O módulo Java produz os mesmos `Finding` das outras linguagens, sem depender do idioma. A avaliação e a apresentação existentes aplicam inglês ou português conforme `--lang` ou o ambiente. O JSON mantém seus nomes de campos, algoritmos e valores internos; traduz apenas os textos previstos pela implementação compartilhada. As descrições dos algoritmos na base continuam em inglês, como no restante do projeto.

A mudança de idioma com `L`, os filtros e a nova varredura com `r` também funcionam com os resultados Java. O teste de integração da TUI executa uma varredura real em segundo plano, troca o idioma durante a espera e verifica a apresentação, os filtros e a nova análise.

## Responsabilidade de cada arquivo

| Arquivo | O que faz e por quê |
| --- | --- |
| `src/modules/java/mod.rs` | Organiza o módulo e exporta `JavaAnalyzer`, seguindo o padrão do analisador JavaScript. |
| `src/modules/java/index.rs` | Implementa `Analyzer`, percorre a árvore, associa inicializações às instâncias corretas, acompanha chaves AES e monta os achados. Mantém linhas e colunas começando em 1 e trechos limitados a 120 caracteres, como o analisador existente. |
| `src/modules/java/keys.rs` | Segue uma chave da sua criação até a operação que a usa: `gerador → par = gerador.generateKeyPair() → par.getPrivate() → assinador.initSign(...)`, inclusive através de casts e aliases. Identifica variáveis pela declaração, não pelo nome, para que `par` em um método não se confunda com `par` em outro. |
| `src/modules/java/bindings.rs` | Resolve imports explícitos, imports com `*`, imports estáticos e nomes completos. Acompanha declarações por escopo, aliases, campos `final` e variáveis locais sem reatribuição. Evita tratar um parâmetro ou uma classe local chamada `MessageDigest` como a API do JDK. |
| `src/modules/java/value.rs` | Lê strings, escapes comuns de Java, números, concatenações e expressões simples de tamanho. Também entende especificações de curva e parâmetros de geração. Limita a avaliação para interromper ciclos ou expansões excessivas de constantes. |
| `src/modules/java/rules.rs` | Mapeia cada API e nome de algoritmo para uma detecção com identificador da base, finalidade, tamanho e confiança. Por exemplo, `SHA256withRSA` vira uma assinatura RSA, enquanto `SHA-256` em `MessageDigest` vira um hash SHA-256. |
| `src/modules/mod.rs` e `src/analysis.rs` | Registram e selecionam o novo analisador no processamento normal do programa. |
| `tests/detect_java.rs` | Verifica detecções, tamanhos, finalidades, localização e casos que não devem gerar alertas. |
| `tests/fixtures/java_crypto` | Contém exemplos Java com usos clássicos e usos classificados como resistentes pela base, para testar o programa completo. |
| `tests/analyze.rs` e `tests/scan.rs` | Verificam a descoberta de arquivos Java, os relatórios, as recomendações e os filtros pela CLI. |
| `src/tui/tests.rs` | Verifica a integração Java com a varredura em segundo plano, as duas páginas da TUI, a tradução, os filtros e a nova varredura. |
| `eval/corpus/java` e `tests/eval.rs` | Definem um corpus anotado e impedem regressões nas métricas e nas limitações documentadas. |
| `eval/results/java.md` e `README.md` | Registram os resultados da avaliação e o suporte disponível. |

## APIs e algoritmos reconhecidos

Os nomes e as transformações seguem a documentação oficial de [nomes de algoritmos Java](https://docs.oracle.com/en/java/javase/25/docs/specs/security/standard-names.html), [Cipher](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/javax/crypto/Cipher.html) e [KeyPairGenerator](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/security/KeyPairGenerator.html).

| API | Usos reconhecidos |
| --- | --- |
| `MessageDigest.getInstance` | MD5, SHA-1, SHA-256, SHA-384, SHA-512 e SHA3-256/384/512. |
| `Mac.getInstance` | HMAC com os hashes reconhecidos, com a finalidade `mac` (autenticação de mensagem), como `createHmac` no JavaScript. |
| `SecretKeyFactory.getInstance` | PBKDF2 (`PBKDF2WithHmacSHA256`, `PBKDF2WithHmacSHA1`...), com a finalidade `key_derivation` e o hash interno como algoritmo, como `pbkdf2` no JavaScript. |
| `Signature.getInstance` | RSA, RSA-PSS, DSA, ECDSA, Ed25519, ML-DSA e SLH-DSA. |
| `Cipher.getInstance` | RSA, incluindo transformações OAEP, AES com tamanho conhecido, Triple DES e ChaCha20. |
| `KeyPairGenerator.getInstance` / `KeyFactory.getInstance` | RSA, RSA-PSS, DSA, EC, DH, Ed25519, X25519, ML-KEM, ML-DSA e SLH-DSA. |
| `KeyAgreement.getInstance` | DH, ECDH e X25519. |
| `KeyGenerator.getInstance` | AES com tamanho conhecido, Triple DES e ChaCha20. |
| `KEM.getInstance` | ML-KEM. |
| `new SecretKeySpec(...)` | Chaves simétricas reconhecidas; para AES, lê o comprimento explícito em bytes e o converte para bits. |

As variantes genéricas `XDH` e `EdDSA` em `KeyPairGenerator` só são mapeadas para X25519 e Ed25519 quando seus parâmetros identificam explicitamente essas curvas. Os números de conjuntos de parâmetros pós-quânticos, como `768` em `ML-KEM-768`, não são apresentados como tamanhos de chave em bits.

Chamadas com um provider explícito, como `Signature.getInstance("SHA256withRSA", "BC")`, passam pelas mesmas regras JCA/JCE. Isso não instala, carrega ou valida o provider e não acrescenta análise das APIs diretas do Bouncy Castle.

## Exemplo: RSA

```java
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.Signature;

class Example {
    void run() throws Exception {
        var generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        KeyPair pair = generator.generateKeyPair();
        var signer = Signature.getInstance("SHA256withRSA");
        signer.initSign(pair.getPrivate());
    }
}
```

A primeira chamada produz um achado RSA com chave de 2048 bits. Gerar uma chave RSA não determina sozinho sua finalidade. Por isso, o analisador segue a chave: `generator` gera `pair`, e `pair.getPrivate()` é passado para `signer.initSign`. A finalidade inferida é `signature`, com confiança `medium`, e a chamada a `Signature.getInstance` produz um achado explícito com confiança `high`.

O avaliador usa essa finalidade para recomendar alternativas de assinatura, como ML-DSA, em vez de um mecanismo de estabelecimento de chaves.

A ligação é feita pela declaração de cada variável. Se um método assina com `pair` e outro cifra com outra variável também chamada `pair`, cada chave recebe a sua finalidade. Quando nenhuma operação recebe a chave, as outras operações do mesmo arquivo servem de palpite com confiança `low`; se elas apontam para finalidades diferentes, a finalidade permanece desconhecida. Uma operação cujo algoritmo não pode ser lido, como `Signature.getInstance(configuracao)`, ainda informa a finalidade da chave que recebe.

## Exemplo: AES sem presumir o tamanho

```java
var generator = KeyGenerator.getInstance("AES");
generator.init(256);
var key = generator.generateKey();
var cipher = Cipher.getInstance("AES/GCM/NoPadding");
cipher.init(Cipher.ENCRYPT_MODE, key);
```

O gerador e o cipher são associados à chave AES de 256 bits. Também é possível obter o tamanho de `new SecretKeySpec(new byte[32], "AES")`: 32 bytes correspondem a 256 bits. O parâmetro de tag de autenticação de GCM não é confundido com o tamanho da chave.

Uma chamada isolada a `Cipher.getInstance("AES/GCM/NoPadding")` não permite diferenciar AES-128 de AES-256. Como a base atual possui entradas separadas para essas variantes, esse uso é omitido quando o tamanho não pode ser determinado. O analisador também evita escolher um tamanho quando encontra inicializações condicionais, conflitantes ou desconhecidas, ou geração anterior à inicialização.

## Como executar

Na raiz do agentq:

```sh
# Analisar um projeto Java; não exige JDK, Maven ou Gradle.
cargo run -- /caminho/do/projeto-java

# Gerar relatório JSON.
cargo run -- /caminho/do/projeto-java --format json -o relatorio.json

# Experimentar os exemplos incluídos.
cargo run -- tests/fixtures/java_crypto --format markdown

# Abrir os exemplos Java na interface interativa em português.
cargo run -- tests/fixtures/java_crypto --tui --lang pt-BR

# Gerar um relatório Java em português.
cargo run -- tests/fixtures/java_crypto --format json --lang pt-BR

# Avaliar o corpus Java e atualizar o relatório de métricas.
cargo run --bin eval -- eval/corpus/java -o eval/results/java.md

# Executar os testes e verificações do projeto.
cargo test --offline
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
```

## Validação e limites

O corpus Java contém 8 arquivos e 49 usos anotados. O analisador encontra 47, sem falsos positivos: precisão de 100%, cobertura de 95,9% e F1 de 97,9%. A finalidade está correta nos 47 usos localizados, incluindo `KeyFlow.java`, em que duas chaves RSA e duas EC do mesmo arquivo só se distinguem pelas variáveis. A busca por palavras, usada como comparação pelo avaliador existente, obtém F1 de 45,2%.

Os dois usos não encontrados estão em `Limitations.java`: um algoritmo vem do retorno de um método e outro vem de uma configuração de ambiente. O corpus é sintético e foi construído junto com o analisador; essas métricas medem regressões nesse conjunto, sem demonstrar desempenho equivalente em projetos reais.

A análise permanece local ao arquivo. Não acompanha dependências entre arquivos, herança, reflexão, fluxos arbitrários entre métodos, APIs diretas de terceiros nem configuração em tempo de execução. Imports com `*` não têm resolução completa de classpath. Strings em text blocks e o pré-processamento Unicode da linguagem não são avaliados. Apenas locais sem reatribuição e campos `final` inicializados podem fornecer valores estáticos.

A leitura de tamanhos em inicializações exige evidência consistente no mesmo bloco da criação da instância. O acompanhamento de chaves segue apenas declarações locais e campos, dentro do arquivo: uma chave recebida por parâmetro, devolvida por outro método ou guardada em uma coleção cai no palpite por arquivo, de confiança `low`. O suporte é limitado aos algoritmos presentes na base; DES simples, Ed448, X448, SHA-224 e SHA-512 truncado, por exemplo, não recebem avaliação.

Ausência de achados não comprova ausência de criptografia vulnerável. Um achado marcado como `safe` representa a classificação de risco quântico do algoritmo na base existente, sem validar modos de operação, nonces, gestão de chaves ou a segurança completa da aplicação.
