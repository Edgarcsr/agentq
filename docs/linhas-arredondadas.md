# Linhas arredondadas na tabela de projetos

Uma variação visual da tabela da página Projetos (colunas PROJETO, STATUS, PROBLEMAS e QUANDO) que
foi experimentada e deixada de lado. Ela fica guardada aqui para o caso de querer voltar.

## O que ela faz

- Cada linha fica bem mais baixa: 26 px de linha, 4 px de respiro em cima e embaixo e 1 px do
  separador, cerca de 33 px no total. Hoje cada linha tem 44 px.
- Só o fundo do hover tem os cantos arredondados (`rounded_lg`).
- O separador entre as linhas continua reto. Ele fica numa caixa de fora, com `border_b_1`, e a
  linha arredondada fica dentro dela. Se o `rounded_lg` e o `border_b_1` ficassem no mesmo
  elemento, a linha de baixo curvaria nas pontas.

## Como aplicar de novo

O arquivo [linhas-arredondadas.patch](linhas-arredondadas.patch) tem só essa mudança, em
`src/gui/workspace.rs`. Da raiz do repositório:

```bash
git apply docs/linhas-arredondadas.patch
```

Para tirar de novo:

```bash
git apply -R docs/linhas-arredondadas.patch
```

O patch foi feito sobre o commit `47a33db` mais a troca do ícone de Agendar para a ampulheta. Se o
`workspace.rs` mudar muito perto da função `projects_page`, use `git apply --3way` ou aplique à
mão. A ideia, em resumo, é esta:

```rust
div().py_1().border_b_1().border_color(pal.border).child(
    h_flex()
        .id(("row", i))
        .h(px(26.))
        .px_4()
        .gap_4()
        .items_center()
        .rounded_lg()
        .cursor_pointer()
        .hover(|s| s.bg(pal.hover))
        // ... as colunas de sempre
)
```
