# Análise de criptografia em Python

Este módulo inicia a etapa de identificação de operações criptográficas em projetos Python. O objetivo é localizar usos de criptografia de chave pública que podem exigir avaliação diante de computadores quânticos, fornecer evidências rastreáveis e preparar dados para uma análise de risco e recomendações posteriores.

## Escopo inicial

A primeira versão procura padrões de chamadas associados às bibliotecas `cryptography` e PyCryptodome:

| Biblioteca | Padrões identificados | Operação classificada |
| --- | --- | --- |
| `cryptography` | `rsa.generate_private_key` | Geração de chave RSA |
| `cryptography` | `ec.generate_private_key` | Geração de chave de curva elíptica |
| `cryptography` | `dh.generate_parameters` | Geração de parâmetros Diffie-Hellman (candidato) |
| PyCryptodome | `RSA.generate` | Geração de chave RSA |
| PyCryptodome | `ECC.generate` | Geração de chave de curva elíptica |
| PyCryptodome | `DSS.new` | Configuração candidata para assinatura DSA/ECDSA |

Os achados incluem linha, biblioteca, algoritmo, tipo de operação, evidência e confiança. `Candidate` indica que uma API de chave ou de assinatura foi encontrada; isso ainda não confirma que a operação criptográfica acontece. `Confirmed` indica que o analisador encontrou `sign`, `verify`, `exchange`, `encrypt` ou `decrypt` em uma variável diretamente atribuída a uma das APIs reconhecidas. É uma confirmação estática do padrão no código, não prova de execução em tempo de execução. A API lexical pode analisar uma string (`analyze_source`) ou ler um arquivo (`analyze_file`). Ela permanece como baseline experimental e não usa a árvore sintática.

A camada sintática Tree-sitter (`syntax::parse_source`) é independente do analisador lexical. Ela identifica nós de importação, chamada e atribuição, conserva seus intervalos de origem e reporta erros sintáticos recuperáveis sem executar o código Python.

## Limitações

O analisador lexical usa busca textual e não resolve imports, aliases, wrappers ou fluxo de dados. A confirmação só acompanha atribuições simples a nomes de variáveis; aliases de chave, retornos de funções e chamadas encadeadas podem não ser relacionados. Chamadas dentro de strings ou comentários inline ainda podem gerar candidatos. Comentários formados por uma linha inteira são ignorados. A ausência de achados não prova que o projeto não usa criptografia.

Em particular, `dh.generate_parameters` gera parâmetros; a regra lexical atual não confirma que um acordo de chaves é executado.

A camada Tree-sitter reconhece a estrutura sintática, mas ainda não faz resolução semântica de aliases nem associação de valores. As posições usam linhas 1-based e colunas baseadas em bytes UTF-8, com fim exclusivo. Erros recuperáveis são devolvidos em `issues` junto das estruturas válidas recuperadas da árvore parcial. Os testes cobrem a classificação das flags `ERROR` e `MISSING`; os exemplos inválidos experimentados até agora produziram `ERROR`, sem um fixture que gere um nó `MISSING` real.

Hash, HMAC, cifras simétricas, configurações TLS genéricas e bibliotecas diferentes das listadas ainda não fazem parte deste escopo. Eles devem ser tratados em regras próprias, pois seu impacto e sua avaliação pós-quântica não são iguais aos de mecanismos de chave pública.

## Uso e evolução

```rust
use quantum_agent::modules::python::{analyze_file, analyze_source};

let findings = analyze_source("key = rsa.generate_private_key(...)");
let findings_from_file = analyze_file("src/app.py")?;
let syntax = quantum_agent::modules::python::syntax::parse_source("value = sign(data)")?;
```

Os resultados são indícios para revisão técnica, não uma garantia de vulnerabilidade nem uma recomendação automática de substituição. A análise sintática fornece evidência estrutural, não uma avaliação de segurança. As próximas etapas devem usar essa camada para fundamentar regras criptográficas, ampliar os testes e avaliar precisão e revocação em projetos de referência. Recomendações devem considerar a finalidade identificada: por exemplo, encapsulamento de chaves e assinaturas digitais exigem alternativas pós-quânticas distintas.