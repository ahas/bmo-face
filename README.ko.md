[English](README.md) | **한국어** | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

# face

BMO의 얼굴, `bmo-face`입니다. 케이스 화면에 띄우는 전체 화면 디스플레이, 얼굴을 그리고 다듬는 편집기, 이 둘을
받쳐 주는 서버, 그리고 브리핑의 얼굴을 읽어 가는 `face/expression` 도구 항목으로 이루어져 있습니다. 하나의
서비스로 127.0.0.1:3217에서 `/bmo-face/` 뒤에 떠 있습니다. BMO에게서 두 가지를 빌려 씁니다. 하나는 웹 계정으로,
편집기는 이 계정으로 로그인하고 디스플레이도 이 계정으로 접속합니다. 다른 하나는 리스너 스트림으로, 모든
브리핑이 그 감정과 BMO의 접속 상태와 함께 이 스트림을 타고 옵니다.

## 구성

| 폴더 | 내용 |
|---|---|
| `core/` | 얼굴의 코어인 Rust 크레이트입니다. 표정 문서의 검사와 예전 형식에서의 변환, 눈 세트와 말하는 입 세트, 기하, 블렌드, 고정 시계의 드라이버를 담고 있으며, 테스트는 `tests/` 아래에, 변환 대상인 예전 문서는 `tests/fixtures/` 아래에 있습니다 |
| `client/` | 화면을 담은 별도의 Bun 프로젝트입니다. `@nuxt/ui` 위에 만든 Nuxt 디스플레이와 편집기, 화면에 보이는 모든 문구를 `i18n/locales/ko.yaml`의 키로 둔 것, `app/console/` 아래의 콘솔 번들 진입점과 `scripts/` 아래의 그 빌드, `vendor/` 아래에 들여온 bmo-ui 릴리스와 `patches/` 아래의 고정 패키지에 대한 레이어 패치, 그리고 `tests/` 아래의 테스트가 있습니다 |
| `deploy/` | 배포가 유지하는 사용자 유닛 `bmo-face.service`, 프록시의 `/bmo-face` 로케이션 두 개, 그리고 `system/` 아래에 사람이 한 번 깔아 두는 네이티브 디스플레이어의 시스템 유닛과 PAM 서비스가 있습니다 |
| `displayer/` | 코어 위에 만든 네이티브 디스플레이어의 Rust 크레이트입니다. logind를 통해 패널을 잡아 CPU로 그리고, 얼굴 서버의 문서와 음성 엔진의 브리핑을 듣고, 소리를 재생하며 시선 소켓을 둡니다. 테스트와 그 픽스처는 `tests/` 아래에 있습니다 |
| `docs/` | 기준 문서입니다. 얼굴과 서버(`face.md`), 얼굴을 그리는 기준이 되는 디자인(`expressions.md`), 배포(`deploy.md`), 개발 벤치(`development.md`)를 다룹니다 |
| `scripts/` | 개발 벤치의 명령(데이터 루트 복사본과 벤치 자체)과 코어의 WebAssembly 빌드입니다 |
| `server/` | 코어 위에 만든 서버의 Rust 크레이트입니다. 표정 문서 준비, 편집기 세션, 서버 라우트, 패스스루, 도구 항목을 담고 있으며, 테스트는 `tests/` 아래에 있습니다 |
| `src/` | 화면이 `#faces`로 가져오는 표정 문서의 TypeScript 정의, `wasm/` 아래의 코어 WebAssembly 빌드와 화면이 `#core`로 가져오는 그 타입 모듈, 그리고 개발용 복사본 모듈이 있습니다 |
| `tests/` | 표정 문서 정의, 코어의 WebAssembly 빌드, 개발용 복사본에 대한 테스트입니다 |

루트에는 `core/`, `displayer/`, `server/`를 멤버로 둔 Cargo 워크스페이스 매니페스트와 그 lockfile, `src/`,
`scripts/`, `tests/`를 담는 루트 Bun 프로젝트의 매니페스트와 lockfile, `tsconfig.json`, `VERSION`,
`testbed.toml`, 두 스키마를 곁에 둔 `tool.toml`, 콘솔 얼굴의 선언으로서 매니페스트 중 얼굴이 정하는 필드를 담은
`console-manifest.json`, `rustfmt.toml`, `korean-ledger.json`, `LICENSE`, `CLAUDE.md`가 있습니다.
`docs/PERSONAL-DATA.md`는 검사 `tests/personal-data.test.ts`를 지정하며, 테스트 스위트가 이를 다른 테스트와 함께
실행합니다.

## 문서

| 문서 | 내용 |
|---|---|
| `docs/face.md` | 디스플레이가 무엇을 언제 보여 주는지, BMO가 무엇을 어떤 경로로 보내는지, 화면이 어떻게 접속하는지, 고정 시계와 드라이버, 표정 문서와 경로를 쓰고 블렌드하는 방법, 얼굴의 코어와 그 인터페이스, 네이티브 디스플레이어, 편집기, 서버 라우트, 콘솔 화면 |
| `docs/expressions.md` | 애니메이션이 그리는 BMO의 얼굴. 프레임 단위로 측정한 내용, 색, 도형을 그리는 방법, 눈 세트, 말하는 입 세트, 애니메이션의 움직임, 기본 얼굴 각각이 그리는 모습 |
| `docs/deploy.md` | 커밋이 실제로 반영되는 과정. 릴리스, 유닛, 생성되는 화면과 콘솔 번들, 도구 항목과 프록시, 그리고 네이티브 디스플레이어의 시스템 유닛과 일회성 설정 |
| `docs/development.md` | 개발 벤치. 데이터 루트 복사본, 그 위에서 실행하는 서버와 화면, 그 뒤에 두는 BMO의 개발용 콘솔, 그리고 벤치용 로그인을 만드는 방법 |
| `docs/PERSONAL-DATA.md` | 어떤 파일에도 들어가면 안 되는 개인정보, 허용되는 값, 그리고 이를 막는 패턴 검사(`tests/personal-data.test.ts`) |

## 빌드와 테스트

```sh
bmo-testbed cargo build --workspace
bmo-testbed cargo test --workspace
bmo-testbed cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
bun run check:types
bun test ./tests
(cd client && bun run check)
```

Cargo 명령은 공유 빌드 디렉터리를 가진 `bmo-testbed`를 거쳐 실행합니다. 그냥 `cargo`를 쓰면 워크스페이스를
체크아웃 자체의 `target/`에 처음부터 다시 컴파일하게 됩니다. 크레이트의 포맷과 린트는 따로 단계를 선언할 필요가
없습니다. 변경이 크레이트를 건드리면 testbed의 Cargo 워크스페이스 검사가 모든 멤버에 대해 이를 실행하고, 변경
검사가 해당 크레이트를 컴파일합니다. 화면은 보고하기 전에 벤치(`docs/development.md`)에서 실제로 동작하는 것을
확인합니다.

## 버전

버전은 `VERSION`, Cargo 워크스페이스의 패키지, 화면의 매니페스트에 같은 번호로 적혀 있습니다. bmo-chat의
`docs/conventions.md` 「Versions」에 적힌 대로, 소유자가 선언한 릴리스에서만 올라갑니다. 계약이 바뀌어도 버전은
움직이지 않으며, 배포는 커밋으로 구별합니다.

## 라이선스

`LICENSE`에 적힌 대로 MIT입니다.
