cargo run --bin pdftotext-runner -- \
--config ~/Library/Application\ Support/nocodo/api.toml \
--pdf ~/Downloads/Sumit_Datta_Tata_1mg_report_3.pdf \
--prompt "I want to extract all text from this PDF, correcting for any errors in the names of lab tests since this is a medical report" \
--allowed-dirs ~/Downloads
