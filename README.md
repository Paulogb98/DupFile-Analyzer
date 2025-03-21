<h1 align="center">DupFile-Analyzer</h1>

<p align="center">
  <a href="https://github.com/Paulogb98/DupFile-Analyzer/stargazers">
    <img src="https://img.shields.io/github/stars/Paulogb98/DupFile-Analyzer.svg?colorA=orange&colorB=orange&logo=github"
         alt="GitHub stars">
  </a>
  <a href="https://github.com/Paulogb98/DupFile-Analyzer/issues">
        <img src="https://img.shields.io/github/issues/Paulogb98/DupFile-Analyzer.svg"
             alt="GitHub issues">
  </a>
  <a href="https://github.com/Paulogb98/DupFile-Analyzer/blob/master/LICENSE">
        <img src="https://img.shields.io/github/license/Paulogb98/DupFile-Analyzer.svg"
             alt="GitHub license">
  </a>
</p>

<br>

## DESCRIÇÃO 🔍

DupFile-Analyzer é uma ferramenta de linha de comando escrita em Rust, projetada para detectar arquivos duplicados em um diretório e seus subdiretórios. Ela calcula o hash SHA-256 de cada arquivo e identifica duplicatas de forma rápida e eficiente, usando processamento paralelo. A comparação é baseada no conteúdo dos arquivos, independentemente de seus nomes ou da localização na estrutura de diretórios.

<br>

## PRINCIPAIS FUNCIONALIDADES ✨

- **Hashing com SHA-256:** Garante alta confiabilidade na identificação de arquivos idênticos.

- **Detecção de Duplicatas:** Encontra arquivos com o mesmo conteúdo, independentemente de nomes ou locais diferentes.

- **Análise Baseada no Conteúdo:** A comparação é feita exclusivamente pelo conteúdo do arquivo (hash SHA-256), ignorando nome, extensão ou caminho. Arquivos idênticos serão detectados como duplicatas, mesmo que estejam armazenados em subdiretórios distintos.

- **Processamento Paralelo (Rayon):** Alta performance ao escanear grandes volumes de arquivos.

- **Barra de Progresso Interativa:** Acompanhe a evolução do processamento de arquivos com uma barra de progresso dinâmica, indicando o tempo estimado e a porcentagem concluída.

- **Saída organizada:** Relatórios claros, com agrupamento de arquivos duplicados por hash.

<br>

## REQUISITOS ⚙️

**Rust:** versão 1.70 ou superior (recomendado)  
**Cargo:** gerenciador de pacotes do Rust (instalado junto com o Rust)

<br>

## INSTALAÇÃO E USO 🚀

### 1. Clone o repositório

```bash
git clone https://github.com/Paulogb98/DupFile-Analyzer.git
cd DupFile-Analyzer
```

<br>

### 2. Instale as dependências

Para compilar e rodar o projeto, basta instalar o Rust e o Cargo, que são os requisitos principais para o funcionamento da aplicação.

<br>

No linux, instale facilmente com:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

<br>

Caso você seja usuário de Windows, siga os passos no [site oficial](https://www.rust-lang.org/tools/install).

<br>

### 3. Compile o projeto

Modo release para máxima performance:

```bash
cargo build --release
```

<br>

O executável estará disponível em:

```bash
target/release/dupfile-analyzer
```

<br>

### 4. Execute o arquivo principal .exe

Nesse caso, existem duas opções. Você pode executar diretamente com 'cargo run', ou rodar o binário compilado:

<br>

> Usando cargo run:

```bash
cargo run --release -- "<DIRETÓRIO>"
```

**OU**

> Usando binário compilado (.exe):

```bash
cd target/release
dupfile-analyzer.exe -- "<DIRETÓRIO>"
```

<br>

E depois...

<br>

> Exemplo Prático:

```bash
cargo run --release -- "C:/Users/SeuUsuario/Documents"
```

**OU**

```bash
dupfile-analyzer.exe -- "C:/Users/SeuUsuario/Documents"
```

<br>

## ESTRUTURA DE DIRETÓRIOS 📁

```plaintext
DupFile-Analyzer/
├── src/
│   ├── main.rs
│   └── utils.rs
├── Cargo.toml
├── Cargo.lock
├── LICENSE
└── README.md
```

<br>

## ASPECTOS TÉCNICOS 🛠️

- ### Cálculo de Hash SHA-256
Cada arquivo é lido em blocos (buffered) para otimizar o uso de memória. O hash SHA-256 é aplicado ao conteúdo de cada arquivo de maneira segura e eficiente.

- ### Processamento Paralelo com Rayon
O uso do Rayon permite que os arquivos sejam processados simultaneamente, dividindo a carga de trabalho entre múltiplos núcleos do processador. Isso melhora significativamente o desempenho, especialmente ao lidar com grandes volumes de dados.

- ### Barra de Progresso e Threads
O programa usa uma thread separada para atualizar a barra de progresso enquanto calcula os hashes dos arquivos de forma paralela. A sincronização entre as threads é garantida usando `Arc<Mutex<ProgressBar>>`, permitindo que o progresso seja mostrado de forma precisa e em tempo real.

- ### Identificação de Duplicatas
Os arquivos são agrupados pelo valor do hash. Se dois ou mais arquivos compartilham o mesmo hash, eles são considerados duplicados, mesmo que seus nomes ou localizações sejam diferentes.

⚠️ **Atenção:** arquivos vazios gerarão sempre o mesmo hash SHA-256 padrão (por exemplo, o hash de um arquivo vazio é e3b0c44298fc1...), e serão listados como duplicados. Isso é comum de acontecer com arquivos __init__.py, caso você programe em Python.

<br>

## EXEMPLO DE SAÍDA 🖥️

```bash
ℹ️  Processando diretório: D:/Caminho/Para/Seus/Arquivos
✔️  1742 arquivos encontrados. Processando...

[00:00:15] [========================================] 1742/1742 (100%)

ℹ️  Foram encontradas 2 duplicatas

hash da0c30d23be40e8e1b1027e453e08a0388c1cd60a2d188088c37b3ef9ec523a1:
  /caminho/para/o/arquivo1.pdf
  /caminho/para/o/arquivo2.pdf

hash e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855:
  /caminho/para/o/arquivo3.png
  /caminho/para/o/arquivo4.png
```

- Os arquivos duplicados são agrupados por hash.
- Cada grupo é separado por linha em branco para facilitar a leitura.

<br>

## CONFIGURAÇÕES DE OTIMIZAÇÃO 🔧

No arquivo Cargo.toml, o projeto está configurado para priorizar a performance em runtime sobre o tempo de compilação. Por isso, as seguintes opções de perfil foram adotadas:

```toml
[profile.release]
opt-level = 3          # Máxima otimização de execução
lto = "fat"            # Link Time Optimization completo
codegen-units = 1      # Uma unidade de compilação para melhor otimização
strip = true           # Remove símbolos de debug do binário final
incremental = false    # Impede o reaproveitamente de partes de compilação anteriores
```

<br>

## CONTRIBUIÇÕES 🤝

Contribuições são bem-vindas! Sinta-se à vontade para abrir uma issue ou enviar um pull request.

<br>

## LICENÇA 📚

Este projeto encontra-se publicado sob os termos de licença MIT. Veja o arquivo LICENSE para mais detalhes.
