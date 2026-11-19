# Windows için Masaüstü Saati

[**English**](README.md)

Windows 11 için Rust ve yerel Win32 API’leriyle yazılmış hafif bir masaüstü saati. Yerel saati 24 saat biçiminde gösterir. Varsayılan olarak dakikada bir güncellenir. Saniyeleri açarsan saniyede bir güncellenir.

## Kullanım

`MasaustuSaati.exe` dosyasını aç. Saat varsayılan olarak ana ekranın üst ortasında görünür. Saati sürükleyerek taşıyabilir, sağ tıklayarak kısa ayarları ve **Options** menüsünü açabilirsin. Arka plan %100 saydamken menüyü açmak için bir rakama sağ tıkla.

## Seçenekler

- **General** bölümünde Windows oturumu açıldığında saatin başlamasını ve saniye gösterimini açıp kapatabilirsin.
- **Appearance** bölümünde yazı tipini seçebilir, saat, dakika ve saniye boyutlarını ayrı ayrı ayarlayabilir, renkleri ve saydamlığı değiştirebilirsin. Canlı önizleme değişiklikleri anında gösterir.
- **Position** bölümünde bağlı ekranlar Windows’taki dizilimleriyle görünür. Saati haritada sürükleyebilir veya bir hizalama seçebilirsin. Kenara tam yaslamak için boşluğu 0 piksel yap. Açıyı girebilir veya 45° kısayolunu kullanabilirsin.

Değişiklikleri uygulamak için **Save**’i seç. Ayarlar `%LOCALAPPDATA%\MasaustuSaati\settings.txt` konumuna kaydedilir. Space Mono Bold yazı tipi, [SIL Open Font License 1.1](assets/Space-Mono-LICENSE.txt) kapsamında uygulamaya dahildir.

## Derleme

Windows üzerinde Rust ve Microsoft C++ Build Tools’u kurduktan sonra:

```powershell
cargo build --locked --release
```

Çalıştırılabilir dosya `target/release/MasaustuSaati.exe` konumuna yazılır.
