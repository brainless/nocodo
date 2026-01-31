use crate::tool_error::ToolError;
use crate::types::{PdfToTextRequest, PdfToTextResponse};
use std::path::Path;
use std::process::Command;

/// Execute pdftotext command to extract text from PDF
pub fn execute_pdftotext(request: PdfToTextRequest) -> Result<PdfToTextResponse, ToolError> {
    // Validate PDF file exists
    let pdf_path = Path::new(&request.file_path);
    if !pdf_path.exists() {
        return Err(ToolError::InvalidInput(format!(
            "PDF file does not exist: {}",
            request.file_path
        )));
    }

    // Build pdftotext command
    let mut cmd = Command::new("pdftotext");

    // Add layout preservation flag (default: true)
    if request.preserve_layout {
        cmd.arg("-layout");
    }

    // Add page range if specified
    if let Some(first_page) = request.first_page {
        cmd.arg("-f").arg(first_page.to_string());
    }
    if let Some(last_page) = request.last_page {
        cmd.arg("-l").arg(last_page.to_string());
    }

    // Add encoding if specified
    if let Some(ref encoding) = request.encoding {
        cmd.arg("-enc").arg(encoding);
    }

    // Add no page breaks flag if specified
    if request.no_page_breaks {
        cmd.arg("-nopgbrk");
    }

    // Add input file
    cmd.arg(&request.file_path);

    // Determine output: file or stdout
    let output_to_stdout = request.output_path.is_none();
    if output_to_stdout {
        // Output to stdout (use "-" as output file)
        cmd.arg("-");
    } else if let Some(ref output_path) = request.output_path {
        cmd.arg(output_path);
    }

    // Execute command
    let output = cmd.output().map_err(|e| {
        ToolError::ExecutionError(format!(
            "Failed to execute pdftotext command. Is pdftotext installed? Error: {}",
            e
        ))
    })?;

    // Check exit status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ToolError::ExecutionError(format!(
            "pdftotext command failed: {}",
            stderr
        )));
    }

    // Build response
    if output_to_stdout {
        let content = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(PdfToTextResponse {
            content: Some(content.clone()),
            output_path: None,
            bytes_written: None,
            success: true,
            message: format!("Successfully extracted {} bytes of text", content.len()),
        })
    } else {
        let output_path = request.output_path.unwrap();
        let bytes_written = std::fs::metadata(&output_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);
        Ok(PdfToTextResponse {
            content: None,
            output_path: Some(output_path.clone()),
            bytes_written: Some(bytes_written),
            success: true,
            message: format!("Successfully wrote {} bytes to {}", bytes_written, output_path),
        })
    }
}

/// Verify that pdftotext is installed
pub fn verify_pdftotext_installation() -> anyhow::Result<String> {
    let output = Command::new("pdftotext")
        .arg("-v")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute 'pdftotext -v'. Is pdftotext installed? Error: {}", e))?;

    // pdftotext -v outputs to stderr
    let version_info = String::from_utf8_lossy(&output.stderr).to_string();

    if version_info.is_empty() {
        anyhow::bail!("pdftotext command did not return version information");
    }

    Ok(version_info)
}

/// Verify that qpdf is installed
pub fn verify_qpdf_installation() -> anyhow::Result<String> {
    let output = Command::new("qpdf")
        .arg("--version")
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute 'qpdf --version'. Is qpdf installed? Error: {}", e))?;

    if !output.status.success() {
        anyhow::bail!(
            "qpdf command failed. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let version_info = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(version_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_pdftotext_installation() {
        // This test will pass if pdftotext is installed
        let result = verify_pdftotext_installation();
        if result.is_ok() {
            println!("pdftotext version: {}", result.unwrap());
        }
    }

    #[test]
    fn test_verify_qpdf_installation() {
        // This test will pass if qpdf is installed
        let result = verify_qpdf_installation();
        if result.is_ok() {
            println!("qpdf version: {}", result.unwrap());
        }
    }
}
