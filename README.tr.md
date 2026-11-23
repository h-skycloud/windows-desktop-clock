# Windows için Masaüstü Saati

[**English**](README.md)

Windows 11 için Rust ve yerel Win32 API’leriyle yazılmış hafif bir masaüstü saati. Yerel saati 24 saat biçiminde gösterir. Varsayılan olarak dakikada bir güncellenir. Saniyeleri açarsan saniyede bir güncellenir.

## Kullanım

`MasaustuSaati.exe` dosyasını aç. Saat varsayılan olarak ana ekranın üst ortasında görünür. Saati sürükleyerek taşıyabilir, sağ tıklayarak kısa ayarları ve **Options** menüsünü açabilirsin. Arka plan %100 saydamken menüyü açmak için bir rakama sağ tıkla.

## Kronometre, geri sayım ve alarmlar

**Options → Timers** bölümünde her panelin yazı tipini, rakam rengini, arka plan rengini, saydamlığını ve görünürlüğünü ayrı ayrı ayarlayıp önizleyebilirsin. %100 saydamlıkta yalnızca yazı görünür. Sayacı başlatmak veya duraklatmak için yazıya tıkla. Bu sekmeden geri sayım süresini ve günlük alarmları da yönetebilirsin. **Save** değişiklikleri uygular. Yeni bir süre kaydettiğinde geri sayım sıfırlanır.

- Saatin altındaki **Stopwatch** veya **Countdown** paneline tıklayarak sayacı başlatabilir, duraklatabilir veya sürdürebilirsin. Sağ tık menüsünden sayacı sıfırlayabilir, yazı tipini ve rengini değiştirebilir ya da paneli gizleyebilirsin.
- **Countdown → Set duration** üzerinden `SS:DD:ss` biçiminde saat, dakika ve saniye gir. Geri sayım bittiğinde bir Windows bildirimi görünür.
- **Alarms → Add daily alarm** üzerinden 24 saat biçiminde yerel saati gir. Alarmın alt menüsünden alarmı açabilir, kapatabilir veya silebilirsin. Alarmlar uygulama çalışırken ve Windows uyanıkken her gün tekrarlanır.

Panel stilleri, geri sayım süresi ve alarmlar kaydedilir. Açık sayaçların ilerlemesi uygulama yeniden açıldığında sıfırlanır.

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
