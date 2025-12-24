use crate::dto::UploadResp;
use crate::prelude::*;
use crate::service::{im_friendship_service, im_group_service};
use crate::{config::UploadConfig, models::User};
use image::EncodableLayout;
use image::{ImageFormat, imageops::FilterType};
use salvo::fs::NamedFile;
use salvo::http::mime;
use salvo::oapi::extract::PathParam;
use salvo::prelude::*;
use std::{io::Cursor, path::PathBuf};
use ulid::Ulid;

/// 初始化上传目录
pub fn init_upload_dir(upload_path: &str) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(upload_path)?;
    Ok(())
}

/// 处理图片：压缩和生成缩略图
async fn process_image(
    image_data: &[u8],
    upload_config: &UploadConfig,
    unique_file_name: &str,
    upload_dir: &PathBuf,
    open_id: &str,
) -> Result<(Option<String>, String), String> {
    let img = match image::load_from_memory(image_data) {
        Ok(img) => img,
        Err(e) => return Err(format!("Failed to load image: {}", e)),
    };

    let original_width = img.width();
    let original_height = img.height();

    let (thumb_width, thumb_height) = if original_width > upload_config.thumbnail_max_width
        || original_height > upload_config.thumbnail_max_height
    {
        let ratio = (upload_config.thumbnail_max_width as f32 / original_width as f32)
            .min(upload_config.thumbnail_max_height as f32 / original_height as f32);
        (
            (original_width as f32 * ratio) as u32,
            (original_height as f32 * ratio) as u32,
        )
    } else {
        (original_width, original_height)
    };

    let thumbnail = img.resize(thumb_width, thumb_height, FilterType::Lanczos3);

    // 确定图片格式
    let format = match PathBuf::from(unique_file_name)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
        Some("png") => ImageFormat::Png,
        Some("webp") => ImageFormat::WebP,
        Some("gif") => ImageFormat::Gif,
        _ => ImageFormat::Jpeg, // 默认使用 JPEG
    };

    let original_path = if upload_config.save_original {
        let original_file_path = upload_dir.join(open_id).join(unique_file_name);
        let mut original_buffer = vec![];

        match format {
            ImageFormat::Jpeg => {
                let mut cursor = Cursor::new(&mut original_buffer);
                if let Err(e) = img.write_to(&mut cursor, ImageFormat::Jpeg) {
                    return Err(format!("保存原图失败: {}", e));
                };
            }
            ImageFormat::Png => {
                let mut cursor = Cursor::new(&mut original_buffer);
                if let Err(e) = img.write_to(&mut cursor, ImageFormat::Jpeg) {
                    return Err(format!("保存原图失败: {}", e));
                };
            }
            ImageFormat::WebP => {
                // WebP 需要特殊处理，这里先转换为 JPEG
                let mut cursor = Cursor::new(&mut original_buffer);
                if let Err(e) = img.write_to(&mut cursor, ImageFormat::Jpeg) {
                    return Err(format!("保存原图失败: {}", e));
                }
            }
            ImageFormat::Gif => {
                // GIF 保持原样（GIF 动画需要特殊处理，这里简化处理）
                // 如果原图是 GIF 且需要处理，可以转换为静态图片
                // 这里为了保持兼容性，直接保存原数据
                original_buffer = image_data.to_vec();
            }
            _ => {
                // 其他格式转换为 JPEG
                let mut cursor = Cursor::new(&mut original_buffer);
                if let Err(e) = img.write_to(&mut cursor, ImageFormat::Jpeg) {
                    return Err(format!("保存原图失败: {}", e));
                }
            }
        }

        if let Err(e) = tokio::fs::write(&original_file_path, &original_buffer).await {
            return Err(format!("写入原图文件失败: {}", e));
        }

        Some(format!("{}/{}", open_id, unique_file_name))
    } else {
        None
    };

    let thumb_file_name = f!("thumb_{}", unique_file_name);
    let thumb_file_path = upload_dir.join(open_id).join(&thumb_file_name);
    let mut thumb_buffer = vec![];
    let mut cursor = Cursor::new(&mut thumb_buffer);

    // 缩略图统一使用 JPEG 格式，质量可配置
    if let Err(e) = thumbnail.write_to(&mut cursor, ImageFormat::Jpeg) {
        return Err(format!("生成缩略图失败: {}", e));
    }

    // 如果启用了压缩，对缩略图进行进一步压缩
    // 注意：image crate 的 write_to 不直接支持质量参数
    // 这里我们使用 resize 已经减少了尺寸，如果需要更激进的压缩，可以考虑使用其他库
    // 但通常 resize 到合适尺寸已经能显著减少文件大小

    if let Err(e) = tokio::fs::write(&thumb_file_path, &thumb_buffer).await {
        return Err(format!("写入缩略图文件失败: {}", e));
    }

    Ok((original_path, f!("{}/{}", open_id, thumb_file_name)))
}

/// 上传文件
#[endpoint(tags("upload"))]
pub async fn upload_file(
    req: &mut Request,
    depot: &mut Depot,
) -> JsonResult<MyResponse<UploadResp>> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let upload_config = crate::config::get().upload.clone();
        let open_id = &from_user.open_id;
        // 确保用户的上传目录存在
        let user_upload_dir = PathBuf::from(&upload_config.path).join(&open_id);
        if let Err(e) = init_upload_dir(user_upload_dir.to_str().unwrap()) {
            error!(error = %e, open_id = %open_id, "创建用户上传目录失败");
            return Err(AppError::internal("创建用户上传目录失败"));
        }

        while let Some(file) = req.file("file").await {
            let file_name = file.name().unwrap_or("unknown").to_string();
            let content_type = file.content_type().unwrap_or(mime::STAR_STAR);

            let is_image = if content_type == mime::IMAGE_STAR {
                true
            } else {
                false
            };

            let (max_size_mb, max_size_bytes) = if is_image {
                (
                    upload_config.max_image_size_mb,
                    upload_config.max_image_size_mb * 1024 * 1024,
                )
            } else {
                (
                    upload_config.max_file_size_mb,
                    upload_config.max_file_size_mb * 1024 * 1024,
                )
            };

            if file.size() > max_size_bytes {
                return Err(AppError::internal(f!("文件大小超过{}MB限制", max_size_mb)));
            }

            // 生成唯一文件名：UUID + 原始扩展名
            let file_path_buf = PathBuf::from(&file_name);
            let extension = file_path_buf
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or(if is_image { "jpg" } else { "bin" });

            let unique_file_name = f!("{}.{}", Ulid::new(), extension);
            let upload_dir = PathBuf::from(&upload_config.path);

            if is_image && upload_config.enable_image_processing {
                let file_data = tokio::fs::read(file.path()).await.map_err(|e| {
                    error!(error = %e, open_id = %open_id, "读取文件失败");
                    AppError::internal("读取文件失败")
                })?;

                match process_image(
                    &file_data,
                    &upload_config,
                    &unique_file_name,
                    &upload_dir,
                    &open_id,
                )
                .await
                {
                    Ok((original_path, thumbnail_path)) => {
                        info!(
                            file_name = %unique_file_name,
                            original = ?original_path,
                            thumbnail = ?thumbnail_path,
                            "图片处理成功"
                        );

                        // 返回文件信息（优先返回缩略图 URL，如果存在）
                        // URL 中包含 open_id 路径，用于权限验证
                        let display_url = f!("/api/upload/{}/{}", open_id, thumbnail_path);

                        return json_ok(MyResponse::success_with_data(
                            "Ok",
                            UploadResp {
                                url: display_url,
                                original_url: original_path
                                    .as_ref()
                                    .map(|o| f!("/api/upload/{}", o)),
                                thumbnail_url: Some(f!("/api/upload/{}", thumbnail_path)),
                                file_name,
                                file_type: content_type.to_string(),
                            },
                        ));
                    }
                    Err(e) => {
                        warn!(error = %e, "图片处理失败，将保存原图");
                        // 如果处理失败，降级为保存原图
                    }
                }
            }
            // 对于非图片文件，或图片处理未启用/失败的情况，直接保存原文件
            let file_path = upload_dir.join(open_id).join(&unique_file_name);

            if let Err(e) = tokio::fs::copy(file.path(), file_path).await {
                error!(file_name = %unique_file_name, error = %e, "保存文件失败");
                return Err(AppError::internal("保存文件失败"));
            }

            info!(
                file_name = %unique_file_name,
                open_id = %open_id,
                size = file.size(),
                file_type = %content_type,
                "文件上传成功"
            );

            return json_ok(MyResponse::success_with_data(
                "Ok",
                UploadResp {
                    url: format!("/api/upload/{}/{}", open_id, file_name),
                    original_url: None,
                    thumbnail_url: None,
                    file_name: unique_file_name,
                    file_type: content_type.to_string(),
                },
            ));
        }

        Err(AppError::internal("未找到文件"))
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}

/// 下载文件
// 路由使用 /download/{open_id}/{file_name}
#[endpoint(tags("upload"))]
pub async fn get_file(req: &mut Request, resp: &mut Response, depot: &mut Depot) -> AppResult<()> {
    if let Ok(from_user) = depot.obtain::<User>() {
        let upload_config = crate::config::get().upload.clone();
        let current_open_id = from_user.open_id.clone();
        let req_open_id = req.params().get("open_id").cloned().unwrap_or_default();
        let req_file_name = req.params().get("file_name").cloned().unwrap_or_default();

        // 安全检查：防止路径遍历攻击
        let file_owner_open_id = req_open_id.replace("..", "").replace("\\", "");
        let file_name = req_file_name.replace("..", "").replace("\\", "");

        info!(
            open_id = %current_open_id,
            req_open_id = %req_open_id,
            req_file_name = %req_file_name,
            "下载文件"
        );

        if file_name.is_empty() {
            return Err(AppError::public("无效的文件路径"));
        }

        // 权限检查：允许文件所有者、好友或同群组成员访问
        if file_owner_open_id != current_open_id {
            // 先检查是否是好友关系
            let is_friend =
                match im_friendship_service::is_friend(&current_open_id, &file_owner_open_id).await
                {
                    Ok(is_friend) => is_friend,
                    Err(e) => {
                        warn!(
                            current_open_id = %current_open_id,
                            file_owner_open_id = %file_owner_open_id,
                            error = ?e,
                            "检查好友关系失败"
                        );
                        false
                    }
                };

            // 如果不是好友，检查是否在同一个群组中
            if !is_friend {
                // 获取当前用户的所有群组
                let current_user_groups =
                    match im_group_service::get_user_groups(&current_open_id).await {
                        Ok(groups) => groups,
                        Err(e) => {
                            warn!(
                                current_open_id = %current_open_id,
                                error = ?e,
                                "获取当前用户群组失败"
                            );
                            vec![]
                        }
                    };

                // 获取文件所有者的所有群组
                let owner_groups =
                    match im_group_service::get_user_groups(&file_owner_open_id).await {
                        Ok(groups) => groups,
                        Err(e) => {
                            warn!(
                                file_owner_open_id = %file_owner_open_id,
                                error = ?e,
                                "获取文件所有者群组失败"
                            );
                            vec![]
                        }
                    };

                // 检查是否有共同的群组
                let has_common_group = current_user_groups.iter().any(|current_group| {
                    owner_groups
                        .iter()
                        .any(|owner_group| current_group.group_id == owner_group.group_id)
                });

                if !has_common_group {
                    warn!(
                        current_open_id = %current_open_id,
                        file_owner_open_id = %file_owner_open_id,
                        "用户尝试访问非好友且非同群组的文件"
                    );
                    return Err(AppError::public("无权访问此文件"));
                }
            }
        }

        let file_path = PathBuf::from(&upload_config.path)
            .join(file_owner_open_id)
            .join(&file_name);

        if !file_path.exists() {
            return Err(AppError::public("文件不存在"));
        }

        // 缓存两个月
        resp.add_header("Cache-Control", "public, max-age=5184000", true)
            .unwrap();

        match NamedFile::builder(file_path)
            //.attached_name("image.jpg")  // 加上这个在浏览器就会变成下载
            .build()
            .await
        {
            Ok(file) => file.send(req.headers(), resp).await,
            Err(_) => resp.render(StatusError::internal_server_error()),
        }

        return Ok(());
    } else {
        Err(AppError::unauthorized("用户未登录"))
    }
}
