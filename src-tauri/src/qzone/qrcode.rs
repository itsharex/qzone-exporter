use anyhow::Result;
use chrono::Local;
use num_bigint::BigUint;
use regex::Regex;
use reqwest::header::{self, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt};

const QRCODE_VALID: &str = "二维码未失效";
const QRCODE_EXPIRED: &str = "二维码已失效";
const QRCODE_VERIFYING: &str = "二维码认证中";
const QRCODE_SUCCESS: &str = "二维码认证成功";
const QRCODE_UNKNOWN: &str = "二维码状态未知";

#[derive(Error, Debug)]
pub enum QRCodeError {
    #[error("网络请求错误, 获取二维码失败!")]
    ReqwestError,

    #[error("解码数据错误, 获取二维码失败!")]
    DecodeError,

    #[error("解析Cookie失败, 获取二维码失败!")]
    GetCookieError,

    #[error("文件读写错误, 保存二维码失败!")]
    FileError,

    #[error("检查二维码扫码状态失败!")]
    CheckQRCodeStatusError,
}

impl serde::Serialize for QRCodeError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: serde::ser::Serializer,
    {
      serializer.serialize_str(self.to_string().as_ref())
    }
  }

#[derive(Debug, Serialize, Deserialize)]
pub enum QRCodeResultCode {
    Valid,     // 二维码未失效
    Expired,   // 二维码已失效
    VERIFYING, // 二维码认证中
    Success,   // 二维码认证成功
    Unknown,   // 二维码状态未知
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QRCodeLoginResult {
    pub code: QRCodeResultCode,
    pub msg: String,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QRCode {
    pub qrcode_path: String, // 二维码图片保存路径
    pub qrsig: String,       // 二维码签名
    pub ptqrtoken: String,   // 二维码token
}

// 计算ptqrtoken
fn get_ptqrtoken(qrsig: &str) -> String {
    let mut e: BigUint = BigUint::from(0_u32);
    for c in qrsig.chars() {
        e += (e.clone() << 5) + c as u32;
    }
    let i = BigUint::from(2147483647_u32);
    (i & e).to_string()
}

// 获取QQ空间登录二维码
#[tauri::command(async)] 
pub async fn get_login_qrcode() -> Result<QRCode, QRCodeError> {
    let mut headers = header::HeaderMap::new();
    headers.insert("user-agent", header::HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .https_only(true)
        .build()
        .map_err(|_| QRCodeError::ReqwestError)?;

    let url = "https://ssl.ptlogin2.qq.com/ptqrshow?appid=549000912&e=2&l=M&s=3&d=72&v=4&t=0.8692955245720428&daid=5&pt_3rd_aid=0";

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| QRCodeError::ReqwestError)?;

    let mut qrsig = String::new();
    for v in response.cookies() {
        if v.name().starts_with("qrsig") {
            qrsig = v.value().to_string();
        }
    }
    if qrsig.is_empty() {
        return Err(QRCodeError::GetCookieError);
    }

    let ptqrtoken = get_ptqrtoken(qrsig.as_str());

    let body = response
        .bytes()
        .await
        .map_err(|_| QRCodeError::DecodeError)?;

    let path = "public/imgs/.qrcode.png".to_string();
    let qrcode_path = match project_root::get_project_root() {
        Ok(p) => p
            .join(&path)
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string()
            .replace("/src-tauri", ""),
        Err(_) => return Err(QRCodeError::FileError),
    };
    println!("{}", qrcode_path);
    let mut file = File::create(&qrcode_path)
        .await
        .map_err(|_| QRCodeError::FileError)?;
    file.write_all(&body)
        .await
        .map_err(|_| QRCodeError::FileError)?;

    Ok(QRCode {
        qrcode_path: path,
        qrsig,
        ptqrtoken,
    })
}

// 检查二维码状态
#[tauri::command(async)] 
pub async fn get_login_result(qrcode: QRCode) -> Result<QRCodeLoginResult, QRCodeError> {
    let mut headers = header::HeaderMap::new();
    let cookie = HeaderValue::from_str(&format!("qrsig={}", qrcode.qrsig))
        .map_err(|_| QRCodeError::CheckQRCodeStatusError)?;
    headers.insert("user-agent", header::HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36"));
    headers.insert("cookie", cookie);

    let cookies_store = reqwest_cookie_store::CookieStore::new(None);
    let cookie_store = reqwest_cookie_store::CookieStoreMutex::new(cookies_store);
    let cookie_store = std::sync::Arc::new(cookie_store);
    let url = format!("https://ssl.ptlogin2.qq.com/ptqrlogin?u1=https%3A%2F%2Fqzs.qq.com%2Fqzone%2Fv5%2Floginsucc.html%3Fpara%3Dizone&ptqrtoken={}&ptredirect=0&h=1&t=1&g=1&from_ui=1&ptlang=2052&action=0-0-{}&js_ver=20032614&js_type=1&login_sig=&pt_uistyle=40&aid=549000912&daid=5&", qrcode.ptqrtoken, Local::now().timestamp_millis());
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .https_only(true)
        .cookie_provider(cookie_store.clone())
        .build()
        .map_err(|_| QRCodeError::CheckQRCodeStatusError)?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| QRCodeError::CheckQRCodeStatusError)?;

    let content = response
        .text()
        .await
        .map_err(|_| QRCodeError::CheckQRCodeStatusError)?;

    if content.contains("二维码未失效") {
        Ok(QRCodeLoginResult {
            code: QRCodeResultCode::Valid,
            msg: QRCODE_VALID.to_string(),
            data: None,
        })
    } else if content.contains("二维码认证中") {
        Ok(QRCodeLoginResult {
            code: QRCodeResultCode::VERIFYING,
            msg: QRCODE_VERIFYING.to_string(),
            data: None,
        })
    } else if content.contains("二维码已失效") {
        Ok(QRCodeLoginResult {
            code: QRCodeResultCode::Expired,
            msg: QRCODE_EXPIRED.to_string(),
            data: None,
        })
    } else if content.contains("登录成功") {
        let regex =
            Regex::new(r"(?P<url>https?://[-A-Za-z0-9+&@#/%?=~_|!:,.;]+[-A-Za-z0-9+&@#/%=~_|])")
                .map_err(|_| QRCodeError::CheckQRCodeStatusError)?;
        let url = match regex.captures(&content) {
            Some(cap) => cap.name("url").unwrap().as_str(),
            None => "",
        };
        if url.is_empty() {
            return Ok(QRCodeLoginResult {
                code: QRCodeResultCode::Unknown,
                msg: QRCODE_UNKNOWN.to_string(),
                data: None,
            });
        }
        let _ = client
            .get(url)
            .send()
            .await
            .map_err(|_| QRCodeError::CheckQRCodeStatusError)?;

        let mut writer = std::fs::File::create("./cookies2.json")
            .map(std::io::BufWriter::new)
            .unwrap();
        let store = cookie_store.lock().unwrap();
        store.save_json(&mut writer).unwrap();
        for item in store.iter_any() {
            println!("{:?}, {:?}", item.name(), item.value());
        }

        Ok(QRCodeLoginResult {
            code: QRCodeResultCode::Success,
            msg: QRCODE_SUCCESS.to_string(),
            data: None,
        })
    } else {
        Ok(QRCodeLoginResult {
            code: QRCodeResultCode::Unknown,
            msg: QRCODE_UNKNOWN.to_string(),
            data: None,
        })
    }
}
